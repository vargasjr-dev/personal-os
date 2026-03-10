# PersonalOS Deployment Guide

Complete guide to deploying PersonalOS on AWS EC2 and accessing it via Vercel.

---

## Architecture Overview

```
┌─────────────────┐
│  Mobile Browser │
│   (Your Phone)  │
└────────┬────────┘
         │ HTTPS
         ▼
┌─────────────────┐
│ Vercel Website  │  personalos.vercel.app
│  (Next.js VNC)  │
└────────┬────────┘
         │ WebSocket
         ▼
┌─────────────────┐
│   AWS EC2       │  t2.micro (Free Tier)
│   - QEMU        │
│   - noVNC       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  PersonalOS     │  Your OS kernel
│   (Bare Metal)  │
└─────────────────┘
```

---

## Part 1: AWS EC2 Setup

### 1. Launch EC2 Instance

**Via AWS Console:**
1. Go to EC2 Dashboard → Launch Instance
2. Choose **Ubuntu Server 22.04 LTS**
3. Instance type: **t2.micro** (free tier eligible)
4. Create/select key pair (download `.pem` file)
5. Security group: Allow ports **22** (SSH), **80** (HTTP), **6080** (WebSocket)
6. Launch instance

**Security Group Rules:**
```
Type        Protocol    Port    Source
SSH         TCP         22      Your IP (or 0.0.0.0/0)
HTTP        TCP         80      0.0.0.0/0
Custom TCP  TCP         6080    0.0.0.0/0
```

### 2. Connect to EC2

```bash
chmod 400 your-key.pem
ssh -i your-key.pem ubuntu@<ec2-public-ip>
```

### 3. Run Setup Script

```bash
# Download setup script
wget https://raw.githubusercontent.com/vargasjr-dev/personal-os/main/deployment/ec2/setup-ec2.sh

# Make executable
chmod +x setup-ec2.sh

# Run setup
./setup-ec2.sh
```

This installs:
- QEMU (x86 emulator)
- noVNC (WebSocket VNC server)
- nginx (reverse proxy)
- Systemd services (auto-start)

### 4. Deploy Your OS Image

**Build the OS locally:**
```bash
cd personal-os
cargo bootimage --release
```

**Upload to EC2:**
```bash
scp -i your-key.pem \
  target/x86_64-personal_os/release/bootimage-personal-os.bin \
  ubuntu@<ec2-ip>:/tmp/
```

**Deploy on EC2:**
```bash
ssh -i your-key.pem ubuntu@<ec2-ip>
/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin
```

### 5. Enable Auto-Start

```bash
sudo systemctl enable personalos-qemu
sudo systemctl enable personalos-novnc
```

### 6. Check Status

```bash
sudo systemctl status personalos-qemu
sudo systemctl status personalos-novnc

# View logs
sudo journalctl -u personalos-qemu -f
```

### 7. Test Access

Open in browser:
```
http://<ec2-public-ip>:6080/vnc.html
```

You should see your OS booting!

---

## Part 2: Vercel Website Deployment

### 1. Install Dependencies

```bash
cd personal-os/web
npm install
```

### 2. Test Locally

```bash
# Create .env.local
echo "NEXT_PUBLIC_EC2_HOST=<your-ec2-ip>" > .env.local

# Run dev server
npm run dev

# Open http://localhost:3000
```

### 3. Deploy to Vercel

**Via Vercel CLI:**
```bash
npm install -g vercel
cd personal-os/web
vercel deploy --prod
```

**Via Vercel Dashboard:**
1. Go to [vercel.com](https://vercel.com)
2. Import Git repository: `vargasjr-dev/personal-os`
3. Root directory: `web`
4. Environment variables:
   - `NEXT_PUBLIC_EC2_HOST` = `<your-ec2-public-ip>`
5. Deploy!

### 4. Configure Custom Domain (Optional)

In Vercel dashboard:
1. Settings → Domains
2. Add `personalos.vargasjr.dev`
3. Update DNS records as instructed

---

## Part 3: GitHub Actions Auto-Deploy

### 1. Add GitHub Secrets

Go to repo Settings → Secrets and variables → Actions:

- `EC2_HOST` = EC2 public IP or hostname
- `EC2_USER` = `ubuntu`
- `EC2_SSH_KEY` = Contents of your `.pem` file

### 2. Enable Workflow

The workflow (`.github/workflows/deploy-to-ec2.yml`) is already in place.

**Trigger:**
- Push to `main` branch
- Manual trigger via Actions tab

### 3. Test Auto-Deploy

```bash
# Make a change
echo "// test" >> src/main.rs

# Commit and push
git add .
git commit -m "Test auto-deploy"
git push

# Check Actions tab on GitHub
# Your OS will be rebuilt and deployed automatically!
```

---

## Part 4: Access from Mobile

### 1. Open Vercel Site

On your phone, open:
```
https://personalos.vercel.app
```

### 2. Interact with OS

- Touch screen → Mouse clicks
- Virtual keyboard → Type commands (when we add keyboard support)
- Pinch to zoom → Scale viewport

---

## Troubleshooting

### OS Won't Boot

**Check QEMU logs:**
```bash
sudo journalctl -u personalos-qemu -n 50
```

**Common issues:**
- Image file corrupted → Re-upload
- Insufficient memory → Increase EC2 instance size
- Wrong file path → Check `/opt/personalos/bootimage-personal-os.bin`

### Can't Connect from Vercel

**Check noVNC status:**
```bash
sudo systemctl status personalos-novnc
netstat -tulpn | grep 6080
```

**Common issues:**
- Port 6080 not open → Update security group
- WebSocket blocked → Check nginx config
- Wrong EC2 host → Verify `NEXT_PUBLIC_EC2_HOST` in Vercel

### GitHub Actions Fails

**Check secrets:**
- Ensure `EC2_SSH_KEY` is the full `.pem` file contents
- Verify `EC2_HOST` is reachable from GitHub runners
- Check `EC2_USER` is `ubuntu`

**SSH connection issues:**
```bash
# Test SSH from local machine first
ssh -i your-key.pem ubuntu@<ec2-host>

# If it works locally, check GitHub Actions logs for specifics
```

---

## Cost Breakdown

### AWS EC2
- **t2.micro**: Free tier (12 months) or ~$8/month
- **Data transfer**: ~$0.09/GB (minimal for VNC)
- **Estimated**: $0-10/month

### Vercel
- **Hobby plan**: Free (100GB bandwidth)
- **Pro plan**: $20/month (if needed)
- **Estimated**: $0-20/month

### Total
- **First year**: $0/month (free tiers)
- **After**: $8-30/month

---

## Advanced: Cloudflare Tunnel (Optional)

For secure HTTPS access without opening ports:

### 1. Install on EC2

```bash
wget https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb
```

### 2. Authenticate

```bash
cloudflared tunnel login
```

### 3. Create Tunnel

```bash
cloudflared tunnel create personalos
cloudflared tunnel route dns personalos os.vargasjr.dev
```

### 4. Run Tunnel

```bash
cloudflared tunnel run personalos --url http://localhost:6080
```

### 5. Update Vercel

Set `NEXT_PUBLIC_EC2_HOST` to:
```
os.vargasjr.dev
```

Now access via HTTPS with no open ports! 🔒

---

## Monitoring & Maintenance

### View OS Logs

```bash
ssh ubuntu@<ec2-ip>
sudo journalctl -u personalos-qemu -f
```

### Restart Services

```bash
sudo systemctl restart personalos-qemu
sudo systemctl restart personalos-novnc
```

### Update OS Image

```bash
# Build new image locally
cargo bootimage --release

# Upload
scp target/.../bootimage-personal-os.bin ubuntu@<ec2-ip>:/tmp/

# Deploy
ssh ubuntu@<ec2-ip> '/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin'
```

### Or use GitHub Actions

Just push to `main` branch → automatic deployment!

---

## Next Steps

1. **Add keyboard input** to the OS kernel
2. **Implement shell** for interactive commands
3. **Add LLM integration** (Anthropic API first)
4. **Build networking stack** for API calls
5. **Optimize performance** for mobile experience

---

**The future of computing is now accessible from your phone.** ⚔️
