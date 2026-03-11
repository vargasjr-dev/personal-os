# Terraform Backend Configuration
# 
# By default, Terraform stores state locally in terraform.tfstate
# For production/team environments, use remote state storage (S3 + DynamoDB)
#
# To enable S3 backend:
# 1. Create S3 bucket: aws s3 mb s3://personalos-terraform-state
# 2. Create DynamoDB table: 
#    aws dynamodb create-table \
#      --table-name personalos-terraform-locks \
#      --attribute-definitions AttributeName=LockID,AttributeType=S \
#      --key-schema AttributeName=LockID,KeyType=HASH \
#      --provisioned-throughput ReadCapacityUnits=5,WriteCapacityUnits=5
# 3. Uncomment the backend block below
# 4. Run: terraform init -migrate-state

# terraform {
#   backend "s3" {
#     bucket         = "personalos-terraform-state"
#     key            = "production/terraform.tfstate"
#     region         = "us-east-1"
#     encrypt        = true
#     dynamodb_table = "personalos-terraform-locks"
#   }
# }

# For GitHub Actions, state is managed per workflow run
# Consider using Terraform Cloud for better state management:
# https://app.terraform.io/
