data "aws_availability_zones" "available" {
  state = "available"
}

locals {
  name = lower("${var.project_name}-${var.environment}")
  azs  = length(var.availability_zones) > 0 ? var.availability_zones : slice(data.aws_availability_zones.available.names, 0, 2)

  public_app_url = var.public_app_base_url != "" ? var.public_app_base_url : (
    var.certificate_arn != "" ? "https://${aws_lb.app.dns_name}" : "http://${aws_lb.app.dns_name}"
  )
  frontend_api_base_url = var.frontend_api_base_url

  backend_image  = var.backend_image != "" ? var.backend_image : "${aws_ecr_repository.backend.repository_url}:${var.backend_image_tag}"
  frontend_image = var.frontend_image != "" ? var.frontend_image : "${aws_ecr_repository.frontend.repository_url}:${var.frontend_image_tag}"

  openai_api_key_secret_arn = var.openai_api_key_secret_arn != "" ? var.openai_api_key_secret_arn : (
    var.openai_api_key != "" ? aws_secretsmanager_secret.openai_api_key[0].arn : ""
  )

  openai_secret_environment = local.openai_api_key_secret_arn == "" ? [] : [
    {
      name      = "OPENAI_API_KEY"
      valueFrom = local.openai_api_key_secret_arn
    }
  ]

  ecs_secret_arns = compact([
    aws_secretsmanager_secret.database_url.arn,
    aws_secretsmanager_secret.primer_activation_code.arn,
    aws_secretsmanager_secret.primer_session_secret.arn,
    local.openai_api_key_secret_arn,
  ])

  common_tags = merge(
    {
      Project     = var.project_name
      Environment = var.environment
      ManagedBy   = "terraform"
    },
    var.tags,
  )
}
