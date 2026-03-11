# GitHub Actions Auto-Deploy Setup

Enable automatic OS deployment to EC2 on every merge to `main`.

---

## Required GitHub Secrets

Go to: **GitHub repo → Settings → Secrets and variables → Actions**

Click **"New repository secret"** and add these **3 secrets**:

### 1. `EC2_HOST`

**Value:** Your EC2 instance public IP or hostname

**Examples:**
- `54.123.45.67`
- `ec2-54-123-45-67.compute-1.amazonaws.com`

**How to find:**
1. Go to AWS EC2 Console
2. Select your instance
3. Copy "Public IPv4 address" or "Public IPv4 DNS"

---

### 2. `EC2_USER`

**Value:** `ubuntu`

(This is the default username for Ubuntu EC2 instances)

---

### 3. `EC2_SSH_KEY`

**Value:** Your entire `.pem` file contents

**How to get it:**

```bash
# On your local machine
cat your-ec2-key.pem
```

**Copy the entire output**, including:
```
-----BEGIN RSA PRIVATE KEY-----
[all the key content]
-----END RSA PRIVATE KEY-----
```

**Paste this entire block** into the secret value.

---

## How to Add Secrets

### Via GitHub Web UI:

1. Go to your repo: `https://github.com/vargasjr-dev/personal-os`
2. Click **Settings**
3. Sidebar: **Secrets and variables** → **Actions**
4. Click **New repository secret**
5. Enter secret name (e.g., `EC2_HOST`)
6. Paste the value
7. Click **Add secret**
8. Repeat for all 3 secrets

### Via GitHub CLI (Optional):

```bash
# Set EC2_HOST
gh secret set EC2_HOST --body "54.123.45.67"

# Set EC2_USER
gh secret set EC2_USER --body "ubuntu"

# Set EC2_SSH_KEY from file
gh secret set EC2_SSH_KEY < your-ec2-key.pem
```

---

## Test Auto-Deploy

Once secrets are added:

### 1. Make a change to the OS

```bash
cd personal-os

# Make a small change
echo "// auto-deploy test" >> src/main.rs

# Commit and push
git add .
git commit -m "Test auto-deploy"
git push origin main
```

### 2. Watch the deployment

1. Go to **Actions** tab on GitHub
2. You'll see a new workflow run: "Deploy PersonalOS to EC2"
3. Click it to watch progress

**Steps it performs:**
1. ✅ Checkout code
2. ✅ Install Rust nightly + bootimage
3. ✅ Build OS (`cargo bootimage --release`)
4. ✅ SSH to EC2
5. ✅ Upload new OS image
6. ✅ Deploy and restart services

### 3. Verify deployment

```bash
# SSH to your EC2
ssh -i your-key.pem ubuntu@<EC2-IP>

# Check service status
sudo systemctl status personalos-qemu

# View logs
sudo journalctl -u personalos-qemu -n 50
```

Access your site - the new OS version should be running!

---

## Troubleshooting

### Workflow fails at "Configure SSH"

**Problem:** SSH key format is wrong

**Fix:**
- Ensure `EC2_SSH_KEY` contains the **entire** `.pem` file
- Include the `-----BEGIN` and `-----END` lines
- No extra spaces or newlines

### Workflow fails at "Deploy to EC2"

**Problem:** Can't connect to EC2

**Fix:**
- Verify `EC2_HOST` is correct (public IP, not private)
- Check EC2 security group allows SSH from `0.0.0.0/0` (port 22)
- Ensure EC2 instance is running

### Workflow succeeds but OS doesn't update

**Problem:** Deployment script not working

**Fix:**
```bash
# SSH to EC2
ssh -i your-key.pem ubuntu@<EC2-IP>

# Check deployment script exists
ls -la /opt/personalos/deploy.sh

# Run manually to see errors
sudo /opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin
```

---

## Workflow File

The workflow is defined in:
```
.github/workflows/deploy-to-ec2.yml
```

**Triggers:**
- Push to `main` branch
- Changes in `src/**` or `Cargo.*`
- Manual trigger via Actions tab

**Runtime:** ~5-10 minutes per deployment

---

## Security Notes

### Why GitHub Secrets?

- ✅ Encrypted at rest
- ✅ Only exposed during workflow runs
- ✅ Not visible in logs
- ✅ Can't be read via API

### SSH Key Safety

- ⚠️ The `.pem` file grants full access to your EC2 instance
- ✅ Never commit it to the repo
- ✅ Store only in GitHub Secrets
- ✅ Rotate keys periodically

### EC2 Security Group

**Recommended rules:**
- Port 22 (SSH): `0.0.0.0/0` (for GitHub Actions)
- Port 80 (HTTP): `0.0.0.0/0` (for nginx)
- Port 6080 (WebSocket): `0.0.0.0/0` (for noVNC)

**Or use Cloudflare Tunnel** to avoid exposing ports!

---

## Manual Deployment (Without GitHub Actions)

If you prefer to deploy manually:

```bash
# Build locally
cargo bootimage --release

# Upload to EC2
scp -i your-key.pem \
  target/x86_64-personal_os/release/bootimage-personal-os.bin \
  ubuntu@<EC2-IP>:/tmp/

# Deploy
ssh -i your-key.pem ubuntu@<EC2-IP> \
  '/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin'
```

---

## Summary Checklist

- [ ] EC2 instance launched and accessible
- [ ] Setup script run on EC2 (`./setup-ec2.sh`)
- [ ] GitHub secret `EC2_HOST` added
- [ ] GitHub secret `EC2_USER` added
- [ ] GitHub secret `EC2_SSH_KEY` added
- [ ] Test commit pushed to `main`
- [ ] Actions workflow succeeded
- [ ] OS accessible at `http://EC2-IP:6080/vnc.html`

Once all checked, **every push to main = automatic deployment!** 🚀
