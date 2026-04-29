use crate::{
    AppState, db,
    domain::{
        DEFAULT_STUDENT_PUBLIC_ID, InfographicExplanationRequest, InfographicRequest,
        LessonStartRequest, LoginRequest, MemoryGraphRequest, MemoryProfileRequest,
        NarrationRequest, RegisterRequest, StagegateRequest,
    },
    memory,
    openai::build_infographic_prompt,
    report_card, voiceover_store,
};
use poem::{
    Route, get, handler, post,
    web::{Data, Json, Path},
};
use serde_json::{Value, json};

pub fn api_routes() -> Route {
    Route::new()
        .at("/health", get(health))
        .at("/api/auth/register", post(register))
        .at("/api/auth/login", post(login))
        .at("/api/students", get(list_students))
        .at(
            "/api/students/:student_id/report-card",
            get(get_report_card),
        )
        .at("/api/students/:student_id", get(get_student))
        .at("/api/students/:student_id/reset", post(reset_student))
        .at("/api/book/:student_id", get(get_book))
        .at("/api/lesson/start", post(start_lesson))
        .at("/api/tutor/respond", post(tutor_respond))
        .at("/api/artifact/infographic", post(infographic))
        .at(
            "/api/artifact/infographic/explain",
            post(explain_infographic),
        )
        .at("/api/narration/speech", post(narration_speech))
        .at("/api/tutor/stagegate", post(stagegate))
        .at("/api/memory/profile", post(memory_profile))
        .at("/api/memory/graph", post(memory_graph))
}

#[handler]
fn health(Data(state): Data<&AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "primerlab-api",
        "textModel": state.openai.text_model(),
        "imageModel": state.openai.image_model(),
        "speechModel": state.openai.speech_model(),
        "speechVoice": state.openai.speech_voice(),
        "hasOpenAiKey": state.openai.has_api_key()
    }))
}

#[handler]
async fn register(
    Data(state): Data<&AppState>,
    Json(request): Json<RegisterRequest>,
) -> Json<Value> {
    match db::register_local_user(&state.db, request).await {
        Ok(student) => Json(json!({
            "student": student,
            "session": session_for_student(&student.student_id)
        })),
        Err(error) => Json(json!({ "error": error })),
    }
}

#[handler]
async fn login(Data(state): Data<&AppState>, Json(request): Json<LoginRequest>) -> Json<Value> {
    match db::login_local_user(&state.db, &request.username, &request.password).await {
        Ok(student) => Json(json!({
            "student": student,
            "session": session_for_student(&student.student_id)
        })),
        Err(error) => Json(json!({ "error": error })),
    }
}

fn session_for_student(student_id: &str) -> Value {
    json!({
        "token": format!("local-demo:{student_id}"),
        "type": "local-demo"
    })
}

#[handler]
async fn get_student(Path(student_id): Path<String>, Data(state): Data<&AppState>) -> Json<Value> {
    let student = match db::find_student(&state.db, &student_id).await {
        Ok(Some(student)) => Some(student),
        Ok(None) => db::find_student(&state.db, DEFAULT_STUDENT_PUBLIC_ID)
            .await
            .ok()
            .flatten(),
        Err(_) => None,
    };

    Json(json!({ "student": student }))
}

#[handler]
async fn reset_student(
    Path(student_id): Path<String>,
    Data(state): Data<&AppState>,
) -> Json<Value> {
    match db::reset_student_learning_state(&state.db, &student_id).await {
        Ok(student) => Json(json!({
            "studentId": student_id,
            "student": student,
            "book": null,
            "reset": {
                "booksDeleted": true,
                "bookEntriesDeleted": true,
                "memoriesDeleted": true,
                "progressDeleted": true,
                "xpDeleted": true
            }
        })),
        Err(error) => Json(json!({
            "studentId": student_id,
            "student": null,
            "book": null,
            "error": error.to_string(),
            "reset": {
                "booksDeleted": false,
                "bookEntriesDeleted": false,
                "memoriesDeleted": false,
                "progressDeleted": false,
                "xpDeleted": false
            }
        })),
    }
}

#[handler]
async fn list_students(Data(state): Data<&AppState>) -> Json<Value> {
    match db::list_students(&state.db).await {
        Ok(students) => Json(json!({ "students": students })),
        Err(error) => Json(json!({ "error": error.to_string(), "students": [] })),
    }
}

#[handler]
async fn get_report_card(
    Path(student_id): Path<String>,
    Data(state): Data<&AppState>,
) -> Json<Value> {
    match report_card::report_card_for_student(&state.db, &student_id, &state.openai).await {
        Ok(Some(report_card)) => Json(json!({
            "studentId": student_id,
            "reportCard": report_card,
        })),
        Ok(None) => Json(json!({
            "studentId": student_id,
            "reportCard": null,
            "error": "Student report card was not found.",
        })),
        Err(error) => Json(json!({
            "studentId": student_id,
            "reportCard": null,
            "error": error.to_string(),
        })),
    }
}

#[handler]
async fn get_book(Path(student_id): Path<String>, Data(state): Data<&AppState>) -> Json<Value> {
    let student = db::find_student(&state.db, &student_id)
        .await
        .ok()
        .flatten();

    match db::book_state_for_student(&state.db, &student_id).await {
        Ok(book) => Json(json!({ "studentId": student_id, "student": student, "book": book })),
        Err(error) => Json(json!({
            "studentId": student_id,
            "student": student,
            "book": null,
            "error": error.to_string(),
        })),
    }
}

#[handler]
async fn start_lesson(
    Data(state): Data<&AppState>,
    Json(request): Json<LessonStartRequest>,
) -> Json<Value> {
    Json(start_lesson_impl(state, request).await)
}

#[handler]
async fn tutor_respond(
    Data(state): Data<&AppState>,
    Json(request): Json<LessonStartRequest>,
) -> Json<Value> {
    Json(start_lesson_impl(state, request).await)
}

async fn start_lesson_impl(state: &AppState, request: LessonStartRequest) -> Value {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());
    let student = match db::get_or_seed_student(&state.db, &student_id).await {
        Ok(student) => student,
        Err(error) => return json!({ "error": error.to_string() }),
    };
    let character_lookup_topic = request.topic.as_deref().or(request.question.as_deref());
    let narrative_characters =
        match db::relevant_narrative_characters(&state.db, &student_id, character_lookup_topic)
            .await
        {
            Ok(characters) => characters,
            Err(error) => {
                println!(
                    "[primerlab-api] character lookup failed for student={student_id}: {error}"
                );
                Vec::new()
            }
        };

    let lesson = match state
        .openai
        .guide_lesson(&student, &narrative_characters, &request)
        .await
    {
        Ok(lesson) => lesson,
        Err(error) => {
            return json!({
                "studentId": student_id,
                "lesson": null,
                "student": student,
                "aiMode": "openai_unavailable",
                "error": error,
            });
        }
    };

    let lesson_topic = lesson
        .get("topic")
        .and_then(Value::as_str)
        .or(request.topic.as_deref())
        .unwrap_or("personalized starting point");
    let updated_student =
        match db::update_progress_after_lesson(&state.db, &student_id, lesson_topic, &lesson).await
        {
            Ok(student) => student,
            Err(_) => student,
        };
    let mut response_student = updated_student;
    let book = match db::update_lesson_book_state(
        &state.db,
        &student_id,
        lesson_topic,
        &request,
        &lesson,
    )
    .await
    {
        Ok(book) => {
            if let Ok(Some(student)) = db::find_student(&state.db, &student_id).await {
                response_student = student;
            }
            Some(book)
        }
        Err(error) => {
            println!(
                "[primerlab-api] current lesson persistence failed for student={student_id}: {error}"
            );
            None
        }
    };

    json!({
        "studentId": student_id,
        "lesson": lesson,
        "student": response_student,
        "book": book
    })
}

#[handler]
async fn infographic(
    Data(state): Data<&AppState>,
    Json(request): Json<InfographicRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());
    let student = match db::get_or_seed_student(&state.db, &student_id).await {
        Ok(student) => student,
        Err(error) => return Json(json!({ "error": error.to_string() })),
    };

    let result = match state.openai.generate_infographic(&student, &request).await {
        Ok(result) => result,
        Err(error) => json!({
            "aiMode": "openai_error",
            "generated": false,
            "error": error,
            "prompt": build_infographic_prompt(&student, &request)
        }),
    };
    let book = match db::update_infographic_book_state(&state.db, &student_id, &request, &result)
        .await
    {
        Ok(book) => Some(book),
        Err(error) => {
            println!(
                "[primerlab-api] current infographic persistence failed for student={student_id}: {error}"
            );
            None
        }
    };

    Json(json!({
        "studentId": student_id,
        "artifact": result,
        "book": book
    }))
}

#[handler]
async fn explain_infographic(
    Data(state): Data<&AppState>,
    Json(request): Json<InfographicExplanationRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());
    let student = match db::get_or_seed_student(&state.db, &student_id).await {
        Ok(student) => student,
        Err(error) => return Json(json!({ "error": error.to_string() })),
    };

    let voiceover_identity = voiceover_store::voiceover_identity(&student_id, &request);
    match db::find_infographic_voiceover(&state.db, &student_id, &voiceover_identity.cache_key)
        .await
    {
        Ok(Some(saved)) => {
            match voiceover_store::read_voiceover_audio(&saved.file_path, &saved.content_type).await
            {
                Ok(audio_data_url) => {
                    return Json(json!({
                        "studentId": student_id,
                        "explanation": voiceover_store::response_from_saved_explanation(
                            &saved.explanation,
                            audio_data_url,
                            &saved.file_path,
                            &saved.content_type,
                        )
                    }));
                }
                Err(error) => {
                    println!(
                        "[primerlab-api] saved infographic voiceover could not be read for student={student_id}: {error}"
                    );
                }
            }
        }
        Ok(None) => {}
        Err(error) => {
            println!(
                "[primerlab-api] saved infographic voiceover lookup failed for student={student_id}: {error}"
            );
        }
    }

    let mut result = match state.openai.explain_infographic(&student, &request).await {
        Ok(result) => result,
        Err(error) => json!({
            "aiMode": "openai_error",
            "generated": false,
            "speechGenerated": false,
            "error": error,
            "model": state.openai.text_model(),
            "speechModel": state.openai.speech_model(),
            "voice": state.openai.speech_voice()
        }),
    };
    if result
        .get("generated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && result
            .get("speechGenerated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        if let Some(audio_data_url) = result
            .get("speech")
            .and_then(|speech| speech.get("audioDataUrl"))
            .and_then(Value::as_str)
        {
            match voiceover_store::write_voiceover_audio(
                &student_id,
                &voiceover_identity.cache_key,
                audio_data_url,
            )
            .await
            {
                Ok(saved_file) => {
                    let persisted_payload =
                        voiceover_store::persisted_explanation_payload(&result, &saved_file);
                    match db::save_infographic_voiceover(
                        &state.db,
                        &student_id,
                        &request,
                        &voiceover_identity.cache_key,
                        &voiceover_identity.image_hash,
                        voiceover_identity.image_length,
                        &persisted_payload,
                        &saved_file.content_type,
                        &saved_file.relative_path,
                    )
                    .await
                    {
                        Ok(_) => {
                            result["cached"] = json!(false);
                            result["persistedVoiceover"] = json!({
                                "saved": true,
                                "cacheKey": voiceover_identity.cache_key,
                                "filePath": saved_file.relative_path,
                                "contentType": saved_file.content_type
                            });
                        }
                        Err(error) => {
                            println!(
                                "[primerlab-api] saved infographic voiceover DB reference failed for student={student_id}: {error}"
                            );
                            result["persistedVoiceover"] = json!({
                                "saved": false,
                                "error": error.to_string()
                            });
                        }
                    }
                }
                Err(error) => {
                    println!(
                        "[primerlab-api] saved infographic voiceover write failed for student={student_id}: {error}"
                    );
                    result["persistedVoiceover"] = json!({
                        "saved": false,
                        "error": error
                    });
                }
            }
        }
    }

    Json(json!({
        "studentId": student_id,
        "explanation": result
    }))
}

#[handler]
async fn narration_speech(
    Data(state): Data<&AppState>,
    Json(request): Json<NarrationRequest>,
) -> Json<Value> {
    let student_id = request.student_id.clone();
    let result = match state.openai.generate_narration(&request).await {
        Ok(result) => result,
        Err(error) => json!({
            "aiMode": "openai_error",
            "generated": false,
            "error": error,
            "model": state.openai.speech_model(),
            "voice": state.openai.speech_voice()
        }),
    };

    Json(json!({
        "studentId": student_id,
        "narration": result
    }))
}

#[handler]
async fn stagegate(
    Data(state): Data<&AppState>,
    Json(request): Json<StagegateRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());
    let student = match db::get_or_seed_student(&state.db, &student_id).await {
        Ok(student) => student,
        Err(error) => return Json(json!({ "error": error.to_string() })),
    };

    let result = match state.openai.grade_stagegate(&student, &request).await {
        Ok(result) => result,
        Err(error) => {
            let fallback_result = json!({
                "passed": false,
                "score": 0.0,
                "rubric": {
                    "accuracy": 0.0,
                    "causalReasoning": 0.0,
                    "vocabulary": 0.0,
                    "transfer": 0.0
                },
                "masteryEvidence": [],
                "gaps": ["The backend stagegate assessor was unavailable."],
                "feedbackToStudent": error,
                "newMemories": []
            });
            let book = match db::append_stagegate_book_entry(
                &state.db,
                &student_id,
                &request,
                &fallback_result,
            )
            .await
            {
                Ok(book) => Some(book),
                Err(error) => {
                    println!(
                        "[primerlab-api] book stagegate persistence failed for student={student_id}: {error}"
                    );
                    None
                }
            };
            return Json(json!({
                "studentId": student_id,
                "result": null,
                "student": student,
                "aiMode": "openai_unavailable",
                "error": fallback_result["feedbackToStudent"].clone(),
                "book": book,
            }));
        }
    };

    let updated_student = match db::update_progress_after_stagegate(
        &state.db,
        &student_id,
        &request,
        &result,
    )
    .await
    {
        Ok(student) => student,
        Err(_) => student,
    };
    let book = match db::append_stagegate_book_entry(&state.db, &student_id, &request, &result)
        .await
    {
        Ok(book) => Some(book),
        Err(error) => {
            println!(
                "[primerlab-api] book stagegate persistence failed for student={student_id}: {error}"
            );
            None
        }
    };

    Json(json!({
        "studentId": student_id,
        "result": result,
        "student": updated_student,
        "book": book
    }))
}

#[handler]
async fn memory_profile(
    Data(state): Data<&AppState>,
    Json(request): Json<MemoryProfileRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());

    match memory::profile_for_student(&state.db, &student_id, request).await {
        Ok(Some(profile)) => Json(json!({
            "studentId": student_id,
            "profile": profile,
        })),
        Ok(None) => Json(json!({
            "studentId": student_id,
            "profile": null,
            "error": "Student memory profile was not found.",
        })),
        Err(error) => Json(json!({
            "studentId": student_id,
            "profile": null,
            "error": error.to_string(),
        })),
    }
}

#[handler]
async fn memory_graph(
    Data(state): Data<&AppState>,
    Json(request): Json<MemoryGraphRequest>,
) -> Json<Value> {
    let student_id = request
        .student_id
        .clone()
        .unwrap_or_else(|| DEFAULT_STUDENT_PUBLIC_ID.to_string());

    match memory::graph_for_student(&state.db, &student_id, request).await {
        Ok(Some(graph)) => Json(json!({
            "studentId": student_id,
            "graph": graph,
        })),
        Ok(None) => Json(json!({
            "studentId": student_id,
            "graph": null,
            "error": "Student memory graph was not found.",
        })),
        Err(error) => Json(json!({
            "studentId": student_id,
            "graph": null,
            "error": error.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::OpenAiClient;
    use poem::{EndpointExt, test::TestClient};
    use sea_orm::{DbBackend, MockDatabase};

    #[tokio::test]
    async fn health_route_returns_schema_stable_service_metadata() {
        let state = AppState {
            db: MockDatabase::new(DbBackend::Postgres)
                .into_connection()
                .into(),
            openai: OpenAiClient::for_tests(None),
        };
        let app = api_routes().data(state);
        let client = TestClient::new(app);

        let response = client.get("/health").send().await;
        response.assert_status_is_ok();
        let payload = response.json().await;
        let body = payload.value().object();

        body.get("ok").assert_bool(true);
        body.get("service").assert_string("primerlab-api");
        body.get("textModel").assert_string("gpt-5.5");
        body.get("imageModel").assert_string("gpt-image-2");
        body.get("speechModel").assert_string("gpt-4o-mini-tts");
        body.get("speechVoice").assert_string("fable");
        body.get("hasOpenAiKey").assert_bool(false);
    }

    #[tokio::test]
    async fn report_card_route_returns_schema_stable_missing_student_payload() {
        let state = AppState {
            db: MockDatabase::new(DbBackend::Postgres)
                .append_query_results([Vec::<crate::entities::student::Model>::new()])
                .into_connection()
                .into(),
            openai: OpenAiClient::for_tests(None),
        };
        let app = api_routes().data(state);
        let client = TestClient::new(app);

        let response = client
            .get("/api/students/student-missing/report-card")
            .send()
            .await;
        response.assert_status_is_ok();
        let payload = response.json().await;
        let body = payload.value().object();

        body.get("studentId").assert_string("student-missing");
        body.get("reportCard").assert_null();
        body.get("error")
            .assert_string("Student report card was not found.");
    }

    #[test]
    fn local_demo_session_tokens_are_stable_and_student_scoped() {
        assert_eq!(
            session_for_student("student-123"),
            json!({
                "token": "local-demo:student-123",
                "type": "local-demo"
            })
        );
    }
}
