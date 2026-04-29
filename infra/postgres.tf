data "aws_ami" "amazon_linux_2023" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-2023.*-kernel-*-${var.postgres_ami_architecture}"]
  }

  filter {
    name   = "architecture"
    values = [var.postgres_ami_architecture]
  }

  filter {
    name   = "root-device-type"
    values = ["ebs"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

resource "aws_instance" "postgres" {
  ami                         = data.aws_ami.amazon_linux_2023.id
  instance_type               = var.postgres_instance_type
  subnet_id                   = aws_subnet.private[0].id
  vpc_security_group_ids      = [aws_security_group.postgres.id]
  iam_instance_profile        = aws_iam_instance_profile.postgres.name
  associate_public_ip_address = false
  key_name                    = var.postgres_key_name != "" ? var.postgres_key_name : null
  disable_api_termination     = var.postgres_disable_api_termination
  user_data_replace_on_change = true

  metadata_options {
    http_endpoint = "enabled"
    http_tokens   = "required"
  }

  root_block_device {
    encrypted             = true
    volume_size           = var.postgres_root_volume_gb
    volume_type           = "gp3"
    delete_on_termination = var.postgres_delete_root_volume_on_termination
  }

  user_data = templatefile("${path.module}/user-data/postgres.sh.tftpl", {
    aws_region             = var.aws_region
    db_name                = var.postgres_db_name
    db_user                = var.postgres_user
    db_password_secret_arn = aws_secretsmanager_secret.postgres_password.arn
    postgres_port          = var.postgres_port
    age_branch             = var.postgres_age_branch
  })

  tags = {
    Name = "${local.name}-postgres"
    Role = "postgres"
  }

  depends_on = [
    aws_route_table_association.private,
    aws_secretsmanager_secret_version.postgres_password,
  ]
}
