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

### GitHub App Credentials (for auto-updating secrets)

3. **GITHUB_APP_ID**
   - GitHub App ID for vargas-jr app
   - Already configured: `1344447`

4. **GITHUB_APP_PRIVATE_KEY_B64**
   - Base64-encoded private key for vargas-jr app
   - Used to mint tokens for updating repository secrets

---

## Hardcoded Configuration

The following are hardcoded in the workflow (no secrets needed):

- **AWS Region:** `us-east-1`
- **Instance Type:** `t2.micro` (free tier)
- **Subdomain:** `os.vargasjr.dev`
- **SSH Keys:** Auto-generated per workflow run

---

## Auto-Updated Secrets

These secrets are automatically updated by the workflow:

- **EC2_HOST** - Set to the new Elastic IP after deployment
- **EC2_USER** - Should be manually set to `ubuntu` once
- **EC2_SSH_KEY** - Not needed (SSH key generated per workflow)

---

## Quick Setup

```bash
# Get your AWS credentials
aws configure list

# Verify GitHub App credentials are set
# GITHUB_APP_ID and GITHUB_APP_PRIVATE_KEY_B64 should already be configured
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
