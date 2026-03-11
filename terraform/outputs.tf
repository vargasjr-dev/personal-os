# Outputs for easy reference and automation

output "elastic_ip" {
  description = "Elastic IP address for PersonalOS"
  value       = aws_eip.personalos.public_ip
}

output "instance_id" {
  description = "EC2 Instance ID"
  value       = aws_instance.personalos.id
}

output "instance_public_dns" {
  description = "EC2 Public DNS"
  value       = aws_instance.personalos.public_dns
}

output "subdomain" {
  description = "Subdomain for PersonalOS"
  value       = var.subdomain
}

output "route53_nameservers" {
  description = "Route 53 nameservers for subdomain delegation"
  value       = aws_route53_zone.personalos.name_servers
}

output "route53_zone_id" {
  description = "Route 53 Hosted Zone ID"
  value       = aws_route53_zone.personalos.zone_id
}

output "security_group_id" {
  description = "Security Group ID"
  value       = aws_security_group.personalos.id
}

output "ssh_command" {
  description = "SSH command to connect"
  value       = "ssh -i ${var.ssh_private_key_path} ubuntu@${aws_eip.personalos.public_ip}"
}

output "vnc_url" {
  description = "Direct noVNC access URL"
  value       = "http://${aws_eip.personalos.public_ip}:6080/vnc.html"
}

output "subdomain_url" {
  description = "Subdomain URL (after DNS delegation)"
  value       = "http://${var.subdomain}"
}

output "deployment_summary" {
  description = "Quick reference for deployment"
  value = <<-EOT
  
  ╔═══════════════════════════════════════════════════════════╗
  ║          PersonalOS Infrastructure Deployed! ✅           ║
  ╚═══════════════════════════════════════════════════════════╝
  
  Instance:      ${aws_instance.personalos.id}
  Elastic IP:    ${aws_eip.personalos.public_ip}
  Subdomain:     ${var.subdomain}
  
  Access:
    SSH:         ssh -i ${var.ssh_private_key_path} ubuntu@${aws_eip.personalos.public_ip}
    VNC (IP):    http://${aws_eip.personalos.public_ip}:6080/vnc.html
    VNC (DNS):   http://${var.subdomain} (after delegation)
  
  Next Steps:
    1. Delegate subdomain at your registrar:
       Add NS records for ${var.subdomain} pointing to:
       ${join("\n       ", aws_route53_zone.personalos.name_servers)}
    
    2. Deploy your OS image:
       cargo bootimage --release
       scp target/.../bootimage-personal-os.bin ubuntu@${aws_eip.personalos.public_ip}:/tmp/
       ssh ubuntu@${aws_eip.personalos.public_ip} '/opt/personalos/deploy.sh /tmp/bootimage-personal-os.bin'
    
    3. Update GitHub Actions secrets:
       EC2_HOST = ${aws_eip.personalos.public_ip}
       EC2_USER = ubuntu
       EC2_SSH_KEY = (contents of ${var.ssh_private_key_path})
  
  EOT
}
