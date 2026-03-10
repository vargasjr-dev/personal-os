#!/bin/bash
set -e

# PersonalOS EC2 Setup Script
# Run this on a fresh Ubuntu 22.04 EC2 instance

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║           PersonalOS EC2 Deployment Setup                ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# Update system
info "Updating system packages..."
sudo apt update && sudo apt upgrade -y

# Install QEMU
info "Installing QEMU..."
sudo apt install -y qemu-system-x86

# Install noVNC and websockify
info "Installing noVNC and websockify..."
sudo apt install -y novnc websockify python3-websockify

# Install nginx (optional reverse proxy)
info "Installing nginx..."
sudo apt install -y nginx

# Create directory for OS images
info "Creating deployment directory..."
sudo mkdir -p /opt/personalos
sudo chown ubuntu:ubuntu /opt/personalos

# Create systemd service for QEMU
info "Creating QEMU systemd service..."
sudo tee /etc/systemd/system/personalos-qemu.service > /dev/null <<'EOF'
[Unit]
Description=PersonalOS QEMU Instance
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/opt/personalos
ExecStart=/usr/bin/qemu-system-x86_64 \
    -drive format=raw,file=/opt/personalos/bootimage-personal-os.bin \
    -m 512M \
    -vnc :1 \
    -serial stdio
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Create systemd service for noVNC
info "Creating noVNC systemd service..."
sudo tee /etc/systemd/system/personalos-novnc.service > /dev/null <<'EOF'
[Unit]
Description=PersonalOS noVNC WebSocket Proxy
After=network.target personalos-qemu.service

[Service]
Type=simple
User=ubuntu
ExecStart=/usr/bin/websockify --web=/usr/share/novnc/ 6080 localhost:5901
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Create nginx config (optional, for SSL/custom domain)
info "Creating nginx configuration..."
sudo tee /etc/nginx/sites-available/personalos > /dev/null <<'EOF'
server {
    listen 80;
    server_name _;

    location / {
        proxy_pass http://localhost:6080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 86400;
    }
}
EOF

sudo ln -sf /etc/nginx/sites-available/personalos /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t && sudo systemctl restart nginx

# Reload systemd
info "Reloading systemd daemon..."
sudo systemctl daemon-reload

# Open firewall (if UFW is enabled)
if sudo ufw status | grep -q "Status: active"; then
    info "Configuring firewall..."
    sudo ufw allow 80/tcp
    sudo ufw allow 6080/tcp
    sudo ufw allow 22/tcp
fi

# Create deployment script
info "Creating deployment script..."
cat > /opt/personalos/deploy.sh <<'EOF'
#!/bin/bash
# Deploy new OS image
set -e

if [ -z "$1" ]; then
    echo "Usage: ./deploy.sh <path-to-bootimage>"
    exit 1
fi

echo "Stopping PersonalOS services..."
sudo systemctl stop personalos-qemu personalos-novnc

echo "Backing up old image..."
if [ -f /opt/personalos/bootimage-personal-os.bin ]; then
    cp /opt/personalos/bootimage-personal-os.bin /opt/personalos/bootimage-personal-os.bin.backup
fi

echo "Copying new image..."
cp "$1" /opt/personalos/bootimage-personal-os.bin

echo "Starting PersonalOS services..."
sudo systemctl start personalos-qemu personalos-novnc

echo "✅ Deployment complete!"
echo "Access at: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):6080/vnc.html"
EOF

chmod +x /opt/personalos/deploy.sh

# Print completion message
echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                                                           ║"
echo "║              Setup Complete! ✅                            ║"
echo "║                                                           ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Next steps:"
echo "  1. Upload your OS image:"
echo "     scp target/.../bootimage-personal-os.bin ubuntu@<ec2-ip>:/tmp/"
echo ""
echo "  2. Deploy the image:"
echo "     /opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin"
echo ""
echo "  3. Start services:"
echo "     sudo systemctl start personalos-qemu"
echo "     sudo systemctl start personalos-novnc"
echo ""
echo "  4. Enable auto-start:"
echo "     sudo systemctl enable personalos-qemu"
echo "     sudo systemctl enable personalos-novnc"
echo ""
echo "  5. Check status:"
echo "     sudo systemctl status personalos-qemu"
echo "     sudo systemctl status personalos-novnc"
echo ""
echo "Access your OS at:"
echo "  http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):6080/vnc.html"
echo ""
