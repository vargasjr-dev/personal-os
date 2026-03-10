import OSViewer from '@/components/OSViewer';

export default function Home() {
  // Get EC2 host from environment variable
  // Set this in Vercel dashboard: NEXT_PUBLIC_EC2_HOST
  const ec2Host = process.env.NEXT_PUBLIC_EC2_HOST || 'localhost';

  return <OSViewer ec2Host={ec2Host} />;
}
