use crate::{
    AppState, db,
    domain::{
        DEFAULT_STUDENT_PUBLIC_ID, InfographicRequest, LessonStartRequest, LoginRequest,
        MemoryGraphRequest, MemoryProfileRequest, NarrationRequest, RegisterRequest,
        StagegateRequest,
    },
    memory,
    openai::build_infographic_prompt,
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
        .at("/api/students/:student_id", get(get_student))
        .at("/api/lesson/start", post(start_lesson))
        .at("/api/tutor/respond", post(tutor_respond))
        .at("/api/artifact/infographic", post(infographic))
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
async fn list_students(Data(state): Data<&AppState>) -> Json<Value> {
    match db::list_students(&state.db).await {
        Ok(students) => Json(json!({ "students": students })),
        Err(error) => Json(json!({ "error": error.to_string(), "students": [] })),
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

    let lesson = match state.openai.guide_lesson(&student, &request).await {
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

    let updated_student =
        match db::update_progress_after_lesson(&state.db, &student_id, &request.topic, &lesson)
            .await
        {
            Ok(student) => student,
            Err(_) => student,
        };

    json!({
        "studentId": student_id,
        "lesson": lesson,
        "student": updated_student
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

    Json(json!({
        "studentId": student_id,
        "artifact": result
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
            return Json(json!({
                "studentId": student_id,
                "result": null,
                "student": student,
                "aiMode": "openai_unavailable",
                "error": error,
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

    Json(json!({
        "studentId": student_id,
        "result": result,
        "student": updated_student
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
