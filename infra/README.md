# Primer AWS Infra

Terraform in this folder provisions a hackathon-friendly AWS deployment for Primer:

- VPC with public ALB subnets and private ECS/Postgres subnets.
- One NAT gateway by default so private services can pull images, reach Secrets Manager, and call OpenAI.
- ECR repositories for frontend and backend images.
- ECS/Fargate services for the Next.js frontend and Rust/Poem backend.
- EC2-hosted Postgres 16 running a Docker image based on `pgvector/pgvector:pg16` with Apache AGE compiled in.
- Secrets Manager values for the generated Postgres password and backend `DATABASE_URL`.
- CloudWatch logs for both app services.

## First Deploy

Use the root deploy script for the normal path:

```bash
./deploy.sh
```

It sets `AWS_PROFILE=logisticchaos` by default, bootstraps Terraform if needed, builds and pushes both images, applies the full stack, and forces ECS to pull the newly pushed tags.

Manual equivalent:

```bash
cd infra
terraform init
terraform apply -var='backend_desired_count=0' -var='frontend_desired_count=0'
```

That creates the network, ECR repositories, ALB, and private Postgres host without starting app tasks before images exist. Log in to ECR and build/push images from the repo root:

```bash
AWS_REGION=ap-southeast-2
BACKEND_REPO="$(terraform -chdir=infra output -raw backend_ecr_repository_url)"
FRONTEND_REPO="$(terraform -chdir=infra output -raw frontend_ecr_repository_url)"
AWS_ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text)"
IMAGE_PLATFORM=linux/arm64

aws ecr get-login-password --region "$AWS_REGION" \
  | docker login --username AWS --password-stdin "$AWS_ACCOUNT_ID.dkr.ecr.$AWS_REGION.amazonaws.com"

docker buildx build \
  --platform "$IMAGE_PLATFORM" \
  -f infra/docker/backend.Dockerfile \
  -t "$BACKEND_REPO:latest" \
  --push .

# Empty NEXT_PUBLIC_API_BASE_URL gives the browser same-origin /api calls through the ALB.
docker buildx build \
  --platform "$IMAGE_PLATFORM" \
  -f infra/docker/frontend.Dockerfile \
  --build-arg NEXT_PUBLIC_API_BASE_URL="" \
  -t "$FRONTEND_REPO:latest" \
  --push .
```

`IMAGE_PLATFORM=linux/arm64` matches the default `ecs_cpu_architecture = "ARM64"`. If you change ECS to `X86_64`, build and push `linux/amd64` images instead.

Then apply the full stack:

```bash
terraform apply
```

On an empty user database, the backend task seeds the Jack demo account on its first startup.
Set `backend_seed_demo_user_on_empty_database = false` if you want a deployment to start without
that demo login.

If you later switch to a separate API origin, rebuild and push the frontend with that build arg, then force a new deployment:

```bash
API_URL="$(terraform -chdir=infra output -raw app_url)"
FRONTEND_REPO="$(terraform -chdir=infra output -raw frontend_ecr_repository_url)"

docker buildx build \
  --platform linux/arm64 \
  -f infra/docker/frontend.Dockerfile \
  --build-arg NEXT_PUBLIC_API_BASE_URL="$API_URL" \
  -t "$FRONTEND_REPO:latest" \
  --push .

aws ecs update-service \
  --region ap-southeast-2 \
  --cluster "$(terraform -chdir=infra output -raw ecs_cluster_name)" \
  --service "$(terraform -chdir=infra output -raw frontend_service_name)" \
  --force-new-deployment
```

## OpenAI Key

The backend keeps working without live AI credentials through its fallback path. To enable OpenAI calls, create a Secrets Manager secret containing only the key string and set:

```hcl
openai_api_key_secret_arn = "arn:aws:secretsmanager:ap-southeast-2:123456789012:secret:/primerlab/dev/openai/api-key-AbCdEf"
```

Do not put OpenAI keys into frontend environment variables.

## Signup Gate

Account creation requires the activation code configured by `primer_activation_code`, which defaults to `X4G6S2HjK`. Terraform stores that value in Secrets Manager and injects it into the backend as `PRIMER_ACTIVATION_CODE`. ECS also receives a generated `PRIMER_SESSION_SECRET` for signing bearer sessions.

## Database Access

Postgres is private and accepts traffic from the ECS security group only. The EC2 instance has SSM Session Manager enabled for emergency debugging. Bootstrap logs are written to:

```text
/var/log/primer-postgres-bootstrap.log
```

Terraform state contains generated database secret material because it manages the password and `DATABASE_URL` secret versions. Use encrypted remote state before using this beyond the MVP.
