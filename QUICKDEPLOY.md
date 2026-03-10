# Quick Deploy Guide

Get PersonalOS running on the web in 15 minutes.

---

## Prerequisites

- AWS account (free tier eligible)
- Vercel account (free hobby plan)
- GitHub account

---

## Step 1: Launch AWS EC2 (5 minutes)

1. Go to [AWS EC2 Console](https://console.aws.amazon.com/ec2/)
2. Click "Launch Instance"
3. Settings:
   - **Name**: PersonalOS
   - **AMI**: Ubuntu Server 22.04 LTS
   - **Instance type**: t2.micro (free tier)
   - **Key pair**: Create new or use existing
   - **Security group**: Allow ports 22, 80, 6080
4. Click "Launch Instance"
5. Wait for "Running" status
6. Copy **Public IPv4 address**

---

## Step 2: Setup EC2 (3 minutes)

```bash
# SSH into EC2
ssh -i your-key.pem ubuntu@<EC2-PUBLIC-IP>

# Download and run setup script
wget https://raw.githubusercontent.com/vargasjr-dev/personal-os/main/deployment/ec2/setup-ec2.sh
chmod +x setup-ec2.sh
./setup-ec2.sh
```

Wait for installation to complete.

---

## Step 3: Build & Deploy OS (3 minutes)

**On your local machine:**

```bash
cd personal-os

# Build the OS
cargo bootimage --release

# Upload to EC2
scp -i your-key.pem \
  target/x86_64-personal_os/release/bootimage-personal-os.bin \
  ubuntu@<EC2-PUBLIC-IP>:/tmp/

# Deploy
ssh -i your-key.pem ubuntu@<EC2-PUBLIC-IP> \
  '/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin'
```

---

## Step 4: Test EC2 Access (1 minute)

Open browser:
```
http://<EC2-PUBLIC-IP>:6080/vnc.html
```

You should see PersonalOS booting! ✅

---

## Step 5: Deploy Vercel Website (3 minutes)

```bash
# Install Vercel CLI
npm install -g vercel

# Deploy the website
cd personal-os/web
npm install
vercel deploy --prod
```

Follow prompts, then:

1. Go to Vercel dashboard
2. Settings → Environment Variables
3. Add: `NEXT_PUBLIC_EC2_HOST` = `<EC2-PUBLIC-IP>`
4. Redeploy

---

## Step 6: Access from Mobile

Open on your phone:
```
https://your-project.vercel.app
```

**Your OS is running in the cloud, accessible from anywhere!** 🎉

---

## Optional: Auto-Deploy with GitHub Actions

### Add Secrets

Go to repo Settings → Secrets → Actions:

1. `EC2_HOST` = EC2 public IP
2. `EC2_USER` = `ubuntu`
3. `EC2_SSH_KEY` = Contents of your `.pem` file

### Push Changes

```bash
# Make a change to your OS
echo "// update" >> src/main.rs

# Commit and push
git add .
git commit -m "Update OS"
git push
```

GitHub Actions will automatically:
- Build your OS
- Deploy to EC2
- Restart services

Check Actions tab for status!

---

## Troubleshooting

**Can't access EC2 on port 6080?**
- Check Security Group allows port 6080
- Verify services: `sudo systemctl status personalos-qemu personalos-novnc`

**Vercel can't connect to EC2?**
- Verify `NEXT_PUBLIC_EC2_HOST` is correct
- Check it's the public IP, not private
- Try accessing http://EC2-IP:6080/vnc.html directly first

**GitHub Actions failing?**
- Verify secrets are set correctly
- Check EC2 SSH key has proper permissions
- Ensure EC2 security group allows SSH from 0.0.0.0/0

---

## Next Steps

- Add custom domain in Vercel
- Set up HTTPS with Cloudflare Tunnel
- Implement keyboard input in OS
- Add LLM integration
- Build networking stack

---

**See [DEPLOYMENT.md](deployment/DEPLOYMENT.md) for full details.**
