resource "random_password" "primer_session_secret" {
  length  = 48
  special = false
}

resource "aws_secretsmanager_secret" "postgres_password" {
  name                    = "/${local.name}/postgres/password"
  recovery_window_in_days = var.secret_recovery_window_in_days

  tags = {
    Name = "${local.name}-postgres-password"
  }
}

resource "aws_secretsmanager_secret_version" "postgres_password" {
  secret_id     = aws_secretsmanager_secret.postgres_password.id
  secret_string = var.postgres_password
}

resource "aws_secretsmanager_secret" "database_url" {
  name                    = "/${local.name}/backend/database-url"
  recovery_window_in_days = var.secret_recovery_window_in_days

  tags = {
    Name = "${local.name}-database-url"
  }
}

resource "aws_secretsmanager_secret_version" "database_url" {
  secret_id = aws_secretsmanager_secret.database_url.id
  secret_string = format(
    "postgres://%s:%s@%s:%d/%s",
    var.postgres_user,
    var.postgres_password,
    aws_instance.postgres.private_ip,
    var.postgres_port,
    var.postgres_db_name,
  )
}

resource "aws_secretsmanager_secret" "openai_api_key" {
  count = var.openai_api_key != "" && var.openai_api_key_secret_arn == "" ? 1 : 0

  name                    = "/${local.name}/openai/api-key"
  recovery_window_in_days = var.secret_recovery_window_in_days

  tags = {
    Name = "${local.name}-openai-api-key"
  }
}

resource "aws_secretsmanager_secret_version" "openai_api_key" {
  count = var.openai_api_key != "" && var.openai_api_key_secret_arn == "" ? 1 : 0

  secret_id     = aws_secretsmanager_secret.openai_api_key[0].id
  secret_string = var.openai_api_key
}

resource "aws_secretsmanager_secret" "primer_activation_code" {
  name                    = "/${local.name}/backend/activation-code"
  recovery_window_in_days = var.secret_recovery_window_in_days

  tags = {
    Name = "${local.name}-activation-code"
  }
}

resource "aws_secretsmanager_secret_version" "primer_activation_code" {
  secret_id     = aws_secretsmanager_secret.primer_activation_code.id
  secret_string = var.primer_activation_code
}

resource "aws_secretsmanager_secret" "primer_session_secret" {
  name                    = "/${local.name}/backend/session-secret"
  recovery_window_in_days = var.secret_recovery_window_in_days

  tags = {
    Name = "${local.name}-session-secret"
  }
}

resource "aws_secretsmanager_secret_version" "primer_session_secret" {
  secret_id     = aws_secretsmanager_secret.primer_session_secret.id
  secret_string = random_password.primer_session_secret.result
}
