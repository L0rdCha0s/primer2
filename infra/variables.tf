variable "aws_region" {
  description = "AWS region to deploy Primer into."
  type        = string
  default     = "ap-southeast-2"
}

variable "project_name" {
  description = "Short project name used in AWS resource names."
  type        = string
  default     = "primerlab"
}

variable "environment" {
  description = "Deployment environment name."
  type        = string
  default     = "dev"
}

variable "tags" {
  description = "Additional tags applied to all taggable resources."
  type        = map(string)
  default     = {}
}

variable "vpc_cidr" {
  description = "CIDR block for the Primer VPC."
  type        = string
  default     = "10.42.0.0/16"
}

variable "availability_zones" {
  description = "Availability zones to use. Leave empty to use the first two available AZs in the region."
  type        = list(string)
  default     = []
}

variable "public_subnet_cidrs" {
  description = "CIDR blocks for public ALB/NAT subnets."
  type        = list(string)
  default     = ["10.42.0.0/24", "10.42.1.0/24"]
}

variable "private_subnet_cidrs" {
  description = "CIDR blocks for private ECS/Postgres subnets."
  type        = list(string)
  default     = ["10.42.10.0/24", "10.42.11.0/24"]
}

variable "enable_nat_gateway" {
  description = "Create a NAT gateway so private ECS tasks and the Postgres bootstrap can reach ECR, Secrets Manager, OpenAI, and GitHub."
  type        = bool
  default     = true
}

variable "allowed_http_cidr_blocks" {
  description = "CIDR blocks allowed to reach the public ALB on HTTP/HTTPS."
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "certificate_arn" {
  description = "Optional ACM certificate ARN. When set, the ALB serves HTTPS and redirects HTTP to HTTPS."
  type        = string
  default     = ""
}

variable "public_app_base_url" {
  description = "Optional public base URL, for example https://primer.example.com. Defaults to the ALB DNS name."
  type        = string
  default     = ""
}

variable "frontend_api_base_url" {
  description = "Optional API base URL to expose to the frontend. Leave empty for same-origin /api calls through the ALB."
  type        = string
  default     = ""
}

variable "backend_image" {
  description = "Optional full backend image URI. Defaults to the managed backend ECR repo plus backend_image_tag."
  type        = string
  default     = ""
}

variable "frontend_image" {
  description = "Optional full frontend image URI. Defaults to the managed frontend ECR repo plus frontend_image_tag."
  type        = string
  default     = ""
}

variable "backend_image_tag" {
  description = "Backend ECR image tag used when backend_image is empty."
  type        = string
  default     = "latest"
}

variable "frontend_image_tag" {
  description = "Frontend ECR image tag used when frontend_image is empty."
  type        = string
  default     = "latest"
}

variable "ecr_force_delete" {
  description = "Allow Terraform to delete non-empty ECR repos on destroy. Keep false for safety."
  type        = bool
  default     = false
}

variable "ecs_cpu_architecture" {
  description = "Fargate CPU architecture for app tasks."
  type        = string
  default     = "ARM64"

  validation {
    condition     = contains(["X86_64", "ARM64"], var.ecs_cpu_architecture)
    error_message = "ecs_cpu_architecture must be X86_64 or ARM64."
  }
}

variable "backend_cpu" {
  description = "Backend task CPU units."
  type        = number
  default     = 512
}

variable "backend_memory" {
  description = "Backend task memory in MiB."
  type        = number
  default     = 1024
}

variable "frontend_cpu" {
  description = "Frontend task CPU units."
  type        = number
  default     = 512
}

variable "frontend_memory" {
  description = "Frontend task memory in MiB."
  type        = number
  default     = 1024
}

variable "backend_desired_count" {
  description = "Desired backend ECS task count."
  type        = number
  default     = 1
}

variable "backend_seed_demo_user_on_empty_database" {
  description = "Seed the Jack demo account when the backend starts against an empty user database."
  type        = bool
  default     = true
}

variable "frontend_desired_count" {
  description = "Desired frontend ECS task count."
  type        = number
  default     = 1
}

variable "backend_container_port" {
  description = "Backend container port."
  type        = number
  default     = 4000
}

variable "frontend_container_port" {
  description = "Frontend container port."
  type        = number
  default     = 3000
}

variable "ecs_health_check_grace_period_seconds" {
  description = "Grace period for ALB health checks while ECS services start."
  type        = number
  default     = 600
}

variable "openai_api_key_secret_arn" {
  description = "Optional existing Secrets Manager secret ARN containing OPENAI_API_KEY. Takes precedence over openai_api_key when set."
  type        = string
  default     = ""
}

variable "openai_api_key" {
  description = "Optional OpenAI API key. When set, Terraform creates a Secrets Manager secret and injects it into the backend."
  type        = string
  default     = ""
  sensitive   = true
}

variable "openai_text_model" {
  description = "OpenAI text model passed to the backend."
  type        = string
  default     = "gpt-5.5"
}

variable "openai_image_model" {
  description = "OpenAI image model passed to the backend."
  type        = string
  default     = "gpt-image-2"
}

variable "openai_tts_model" {
  description = "OpenAI speech model passed to the backend."
  type        = string
  default     = "gpt-4o-mini-tts"
}

variable "primer_activation_code" {
  description = "Activation code required by the backend for account creation."
  type        = string
  default     = "X4G6S2HjK"
  sensitive   = true
}

variable "primer_session_ttl_seconds" {
  description = "Signed session token lifetime in seconds."
  type        = number
  default     = 604800
}

variable "postgres_db_name" {
  description = "Primer Postgres database name."
  type        = string
  default     = "primerlab"
}

variable "postgres_user" {
  description = "Primer Postgres application user."
  type        = string
  default     = "primerlab"
}

variable "postgres_password" {
  description = "Primer Postgres application password. Store this in an ignored tfvars file, not in source."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.postgres_password) >= 16
    error_message = "postgres_password must be at least 16 characters."
  }
}

variable "postgres_port" {
  description = "Postgres port."
  type        = number
  default     = 5432
}

variable "postgres_instance_type" {
  description = "EC2 instance type for Postgres."
  type        = string
  default     = "t3.small"
}

variable "postgres_ami_architecture" {
  description = "Architecture for the Amazon Linux 2023 Postgres EC2 AMI."
  type        = string
  default     = "x86_64"

  validation {
    condition     = contains(["x86_64", "arm64"], var.postgres_ami_architecture)
    error_message = "postgres_ami_architecture must be x86_64 or arm64."
  }
}

variable "postgres_root_volume_gb" {
  description = "Encrypted root volume size for the Postgres EC2 instance. Postgres data lives on this volume."
  type        = number
  default     = 40
}

variable "postgres_delete_root_volume_on_termination" {
  description = "Whether to delete the Postgres root/data volume when the EC2 instance is terminated."
  type        = bool
  default     = false
}

variable "postgres_disable_api_termination" {
  description = "Enable EC2 termination protection for the Postgres instance."
  type        = bool
  default     = false
}

variable "postgres_key_name" {
  description = "Optional EC2 key pair name for emergency SSH access. No SSH ingress is opened by default; use SSM Session Manager."
  type        = string
  default     = ""
}

variable "postgres_age_branch" {
  description = "Apache AGE branch to compile into the Postgres container."
  type        = string
  default     = "PG16"
}

variable "postgres_admin_cidr_blocks" {
  description = "Optional extra CIDR blocks allowed to connect to Postgres. Prefer leaving this empty and using app-only access."
  type        = list(string)
  default     = []
}

variable "secret_recovery_window_in_days" {
  description = "Secrets Manager recovery window for generated secrets."
  type        = number
  default     = 7
}
