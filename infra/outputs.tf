output "app_url" {
  description = "Public URL for the deployed app."
  value       = local.public_app_url
}

output "alb_dns_name" {
  description = "Application Load Balancer DNS name."
  value       = aws_lb.app.dns_name
}

output "backend_ecr_repository_url" {
  description = "Backend ECR repository URL."
  value       = aws_ecr_repository.backend.repository_url
}

output "frontend_ecr_repository_url" {
  description = "Frontend ECR repository URL."
  value       = aws_ecr_repository.frontend.repository_url
}

output "ecs_cluster_name" {
  description = "ECS cluster name."
  value       = aws_ecs_cluster.main.name
}

output "backend_service_name" {
  description = "Backend ECS service name."
  value       = aws_ecs_service.backend.name
}

output "frontend_service_name" {
  description = "Frontend ECS service name."
  value       = aws_ecs_service.frontend.name
}

output "postgres_private_ip" {
  description = "Private IP of the EC2 Postgres host."
  value       = aws_instance.postgres.private_ip
}

output "postgres_instance_id" {
  description = "EC2 instance ID of the Postgres host."
  value       = aws_instance.postgres.id
}

output "database_url_secret_arn" {
  description = "Secrets Manager ARN containing DATABASE_URL for the backend."
  value       = aws_secretsmanager_secret.database_url.arn
}

output "postgres_password_secret_arn" {
  description = "Secrets Manager ARN containing the generated Postgres password."
  value       = aws_secretsmanager_secret.postgres_password.arn
}

output "primer_activation_code_secret_arn" {
  description = "Secrets Manager ARN containing the Primer signup activation code."
  value       = aws_secretsmanager_secret.primer_activation_code.arn
}

output "primer_session_secret_arn" {
  description = "Secrets Manager ARN containing the generated Primer session signing secret."
  value       = aws_secretsmanager_secret.primer_session_secret.arn
}

output "openai_api_key_secret_arn" {
  description = "Secrets Manager ARN containing OPENAI_API_KEY, when managed by this stack or provided as an external ARN."
  value       = local.openai_api_key_secret_arn
  sensitive   = true
}
