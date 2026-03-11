#!/bin/bash
set -e

# PersonalOS EC2 User Data - Auto-setup on launch

# Wait for cloud-init to finish
cloud-init status --wait

# Download and execute setup script
cd /tmp
wget -O setup-ec2.sh "${setup_script_url}"
chmod +x setup-ec2.sh
./setup-ec2.sh

# Mark setup as complete
echo "PersonalOS EC2 setup completed at $(date)" > /opt/personalos/setup-complete.txt
