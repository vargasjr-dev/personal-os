# GitHub Secrets Configuration

For automated Terraform deployment via GitHub Actions, configure these secrets in your repository:

**Settings → Secrets → Actions → New repository secret**

---

## Required Secrets

### AWS Credentials

1. **AWS_ACCESS_KEY_ID**
   - Your AWS access key ID
   - Get from: AWS Console → IAM → Users → Security credentials
   - Required permissions: EC2, VPC, Route53

2. **AWS_SECRET_ACCESS_KEY**
   - Your AWS secret access key
   - Generated when you create the access key

3. **AWS_REGION**
   - Default: `us-east-1`
   - Or your preferred AWS region

### Terraform Variables

4. **TF_INSTANCE_TYPE**
   - EC2 instance type
   - Default: `t2.micro` (free tier)
   - Options: `t2.micro`, `t2.small`, `t3.medium`, etc.

5. **TF_SUBDOMAIN**
   - Your subdomain for PersonalOS
   - Example: `os.vargasjr.dev`

### SSH Keys

6. **TF_SSH_PUBLIC_KEY**
   - Contents of your SSH public key
   - Get it: `cat ~/.ssh/id_rsa.pub`
   - Used to create AWS key pair for EC2 access

7. **TF_SSH_PRIVATE_KEY**
   - Contents of your SSH private key
   - Get it: `cat ~/.ssh/id_rsa`
   - ⚠️ Keep this secret! Used for deployment only

### GitHub Token (for auto-updating secrets)

8. **GH_PAT** (GitHub Personal Access Token)
   - Used to auto-update `EC2_HOST` secret after deployment
   - Create at: GitHub → Settings → Developer settings → Personal access tokens → Tokens (classic)
   - Scopes needed: `repo` (full control)
   - Alternative: Use fine-grained token with `secrets: write` permission

---

## Optional: Existing EC2 Secrets

If you already have these for manual deployment, they'll be auto-updated:

- **EC2_HOST** - Auto-updated to new Elastic IP
- **EC2_USER** - Should be `ubuntu`
- **EC2_SSH_KEY** - Should match `TF_SSH_PRIVATE_KEY`

---

## Quick Setup Script

```bash
# Generate SSH key if you don't have one
ssh-keygen -t rsa -b 4096 -f ~/.ssh/personalos_deploy -N ""

# Get your AWS credentials
aws configure list

# Copy values to clipboard (macOS)
cat ~/.ssh/personalos_deploy.pub | pbcopy  # TF_SSH_PUBLIC_KEY
cat ~/.ssh/personalos_deploy | pbcopy      # TF_SSH_PRIVATE_KEY

# Or print them (Linux)
echo "=== TF_SSH_PUBLIC_KEY ==="
cat ~/.ssh/personalos_deploy.pub
echo ""
echo "=== TF_SSH_PRIVATE_KEY ==="
cat ~/.ssh/personalos_deploy
```

---

## Security Best Practices

1. **Use separate AWS IAM user for CI/CD**
   - Don't use your root account
   - Give it minimal required permissions:
     ```json
     {
       "Version": "2012-10-17",
       "Statement": [
         {
           "Effect": "Allow",
           "Action": [
             "ec2:*",
             "route53:*",
             "elasticloadbalancing:*"
           ],
           "Resource": "*"
         }
       ]
     }
     ```

2. **Rotate keys regularly**
   - Generate new AWS access keys every 90 days
   - Update GitHub secrets

3. **Use GitHub Environments** (optional)
   - Settings → Environments → New environment
   - Add protection rules (require approval for production)
   - Assign secrets to specific environments

4. **Enable secret scanning**
   - GitHub automatically scans for leaked secrets
   - Settings → Security → Secret scanning

---

## Verification

After adding secrets, verify they're set:

```bash
# Trigger the workflow manually
gh workflow run terraform-deploy.yml

# Check the run
gh run list --workflow=terraform-deploy.yml
gh run view <run-id>
```

Or push a change to `terraform/` directory to trigger automatically.

---

## Troubleshooting

**"Error: Invalid AWS credentials"**
- Verify `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are correct
- Check IAM user has required permissions

**"Error: SSH key invalid"**
- Ensure `TF_SSH_PUBLIC_KEY` starts with `ssh-rsa` or `ssh-ed25519`
- No extra whitespace or line breaks

**"Error: Could not update EC2_HOST secret"**
- Verify `GH_PAT` has `repo` scope
- Check token hasn't expired

**State lock errors**
- Enable S3 backend with DynamoDB locking (see `backend.tf`)
- Or ensure only one workflow runs at a time

---

## Next Steps

After secrets are configured:

1. Commit a change to `terraform/` and push
2. GitHub Actions will automatically deploy infrastructure
3. Check Actions tab for deployment status
4. Get Elastic IP from workflow output
5. Delegate DNS at your registrar
