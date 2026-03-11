variable "aws_region" {
  description = "AWS region to deploy to"
  type        = string
  default     = "us-east-1"
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t2.micro" # Free tier eligible
}

variable "subdomain" {
  description = "Subdomain for PersonalOS (e.g., os.vargasjr.dev)"
  type        = string
  default     = "os.vargasjr.dev"
}

variable "ssh_public_key_path" {
  description = "Path to SSH public key for EC2 access"
  type        = string
  default     = "~/.ssh/id_rsa.pub"
}

variable "ssh_private_key_path" {
  description = "Path to SSH private key (for display in outputs)"
  type        = string
  default     = "~/.ssh/id_rsa"
}
