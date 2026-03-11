# PersonalOS Terraform Infrastructure

This directory contains Terraform configuration to provision the full PersonalOS AWS infrastructure:

- **EC2 instance** (Ubuntu 22.04 LTS)
- **Elastic IP** (stable public address)
- **Security Groups** (SSH, HTTP, HTTPS, noVNC)
- **Route 53 Hosted Zone** for `os.vargasjr.dev` subdomain
- **A Record** pointing subdomain → Elastic IP

---

## Prerequisites

1. **AWS CLI configured** with credentials:
   ```bash
   aws configure
   ```

2. **Terraform installed** (>= 1.0):
   ```bash
   # macOS
   brew install terraform
   
   # Linux
   wget https://releases.hashicorp.com/terraform/1.7.0/terraform_1.7.0_linux_amd64.zip
   unzip terraform_1.7.0_linux_amd64.zip
   sudo mv terraform /usr/local/bin/
   ```

3. **SSH key pair** (if you don't have one):
   ```bash
   ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa -N ""
   ```

---

## Quick Start

### 1. Initialize Terraform

```bash
cd terraform
terraform init
```

### 2. Configure Variables

```bash
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your preferences
```

### 3. Plan Deployment

```bash
terraform plan
```

Review the resources that will be created.

### 4. Deploy

```bash
terraform apply
```

Type `yes` when prompted.

**This will create:**
- EC2 instance with PersonalOS setup
- Elastic IP
- Route 53 hosted zone for `os.vargasjr.dev`
- DNS A record

### 5. Delegate Subdomain (REQUIRED)

Terraform will output Route 53 nameservers. You need to delegate the subdomain from your registrar:

```bash
terraform output route53_nameservers
```

**At your domain registrar (e.g., Namecheap, GoDaddy, Cloudflare):**

Add **NS records** for `os.vargasjr.dev`:

```
os.vargasjr.dev  NS  ns-123.awsdns-12.com.
os.vargasjr.dev  NS  ns-456.awsdns-45.net.
os.vargasjr.dev  NS  ns-789.awsdns-78.org.
os.vargasjr.dev  NS  ns-012.awsdns-01.co.uk.
```

(Use the actual values from Terraform output)

**Alternative (simpler but less flexible):**
Instead of NS records, add an A record directly:
```
os.vargasjr.dev  A  <elastic_ip_from_terraform_output>
```

This skips Route 53 entirely but means you lose Route 53 features (health checks, traffic routing, etc.).

---

## Verify Deployment

### Check Instance Status

```bash
terraform output ssh_command
# Example: ssh -i ~/.ssh/id_rsa ubuntu@54.123.45.67

# SSH into instance
$(terraform output -raw ssh_command)

# Check services
sudo systemctl status personalos-qemu personalos-novnc
```

### Access noVNC

```bash
# Direct IP access
terraform output vnc_url
# Opens: http://54.123.45.67:6080/vnc.html

# Via subdomain (after DNS propagation)
terraform output subdomain_url
# Opens: http://os.vargasjr.dev
```

### Check DNS Propagation

```bash
# Wait 5-10 minutes after NS delegation, then:
dig os.vargasjr.dev

# Should return the Elastic IP
```

---

## Deploy PersonalOS Image

After infrastructure is up, deploy your OS:

```bash
# Build locally
cd ..
cargo bootimage --release

# Get Elastic IP
ELASTIC_IP=$(cd terraform && terraform output -raw elastic_ip)

# Deploy
scp target/x86_64-personal_os/release/bootimage-personal-os.bin \
  ubuntu@$ELASTIC_IP:/tmp/

ssh ubuntu@$ELASTIC_IP \
  '/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin'
```

---

## Update GitHub Actions Secrets

After Terraform deployment, update GitHub Actions secrets:

```bash
cd terraform

# Get values
terraform output elastic_ip
terraform output -raw ssh_command

# Add to GitHub repo: Settings → Secrets → Actions
```

1. `EC2_HOST` = Elastic IP from output
2. `EC2_USER` = `ubuntu`
3. `EC2_SSH_KEY` = Contents of your private key:
   ```bash
   cat ~/.ssh/id_rsa
   ```

---

## Destroy Infrastructure

To tear down everything:

```bash
terraform destroy
```

**⚠️ Warning:** This will:
- Delete the EC2 instance
- Release the Elastic IP
- Delete the Route 53 hosted zone
- All data will be lost

---

## Costs

**With default settings (t2.micro):**
- EC2 instance: **Free tier** (750 hours/month for 12 months)
- Elastic IP: **Free** (while associated with running instance)
- Route 53 hosted zone: **$0.50/month**
- Data transfer: **Free tier** (15 GB/month out)

**Total: ~$0.50/month** (after free tier)

---

## Customization

### Use a Larger Instance

Edit `terraform.tfvars`:
```hcl
instance_type = "t3.medium"
```

### Change Region

```hcl
aws_region = "us-west-2"
```

### Multiple Environments

```bash
# Create workspaces
terraform workspace new production
terraform workspace new staging

# Switch between them
terraform workspace select production
terraform apply
```

---

## Troubleshooting

### "Invalid credentials" error

```bash
aws configure
# Enter your AWS Access Key ID and Secret
```

### SSH key not found

```bash
# Generate new key
ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa

# Update terraform.tfvars
ssh_public_key_path = "~/.ssh/id_rsa.pub"
```

### DNS not resolving

- Wait 5-10 minutes for DNS propagation
- Verify NS records at registrar: `dig NS os.vargasjr.dev`
- Check Route 53 console for hosted zone

### Can't access noVNC

- Verify security group allows port 6080: `terraform show | grep 6080`
- Check EC2 public IP: `terraform output elastic_ip`
- Test direct access: `curl http://<ELASTIC_IP>:6080`

---

## Next Steps

1. Set up HTTPS with Let's Encrypt/Cloudflare
2. Automate OS builds with GitHub Actions
3. Add monitoring/alerting
4. Set up automated backups
5. Implement CI/CD pipeline

See [../QUICKDEPLOY.md](../QUICKDEPLOY.md) for full deployment guide.
