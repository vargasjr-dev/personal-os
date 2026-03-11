#!/bin/bash
set -e

# PersonalOS Terraform Deployment Helper Script

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

cd "$(dirname "$0")"

if [ ! -f "terraform.tfvars" ]; then
    warn "terraform.tfvars not found. Creating from example..."
    cp terraform.tfvars.example terraform.tfvars
    info "Please edit terraform.tfvars and configure your settings."
    info "Then run this script again."
    exit 0
fi

info "Initializing Terraform..."
terraform init

info "Planning deployment..."
terraform plan -out=tfplan

echo ""
read -p "Apply this plan? (yes/no): " answer
if [ "$answer" != "yes" ]; then
    info "Deployment cancelled."
    exit 0
fi

info "Applying Terraform configuration..."
terraform apply tfplan

echo ""
info "Deployment complete!"
terraform output deployment_summary

info "Terraform outputs saved. Run 'terraform output' to see them again."
