'use client';

import { useState, useEffect } from 'react';

export default function OSViewer({ ec2Host }: { ec2Host: string }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [loadTimeout, setLoadTimeout] = useState(false);

  const vncUrl = `http://${ec2Host}:6080/vnc.html?autoconnect=true&resize=scale`;

  // Timeout after 10 seconds if iframe doesn't load
  useEffect(() => {
    const timer = setTimeout(() => {
      if (loading) {
        setLoadTimeout(true);
        setError('Cannot connect to EC2 instance');
      }
    }, 10000);

    return () => clearTimeout(timer);
  }, [loading]);

  // Check if EC2 host is configured
  useEffect(() => {
    if (!ec2Host || ec2Host === 'localhost') {
      setError('EC2 host not configured');
      setLoading(false);
    }
  }, [ec2Host]);

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-black p-4">
      <div className="w-full max-w-6xl">
        <div className="mb-6 text-center">
          <h1 className="text-4xl font-bold text-green-400 mb-2">
            PersonalOS - Live Demo ⚔️
          </h1>
          <p className="text-gray-400">
            Assistant-Native Operating System
          </p>
          
          {error ? (
            <div className="mt-4 p-4 bg-red-900/30 border border-red-500 rounded-lg">
              <p className="text-red-400 font-semibold">❌ {error}</p>
              <div className="mt-3 text-sm text-gray-300 text-left max-w-2xl mx-auto">
                <p className="font-semibold mb-2">Setup Required:</p>
                <ol className="list-decimal list-inside space-y-1">
                  <li>Launch an AWS EC2 instance (t2.micro, Ubuntu 22.04)</li>
                  <li>Run the setup script: <code className="bg-gray-800 px-2 py-1 rounded">./deployment/ec2/setup-ec2.sh</code></li>
                  <li>Build and deploy your OS image</li>
                  <li>Update <code className="bg-gray-800 px-2 py-1 rounded">NEXT_PUBLIC_EC2_HOST</code> in Vercel</li>
                </ol>
                <p className="mt-3">
                  <a 
                    href="https://github.com/vargasjr-dev/personal-os/blob/main/QUICKDEPLOY.md" 
                    target="_blank" 
                    rel="noopener noreferrer"
                    className="text-green-400 hover:underline"
                  >
                    📖 Full deployment guide →
                  </a>
                </p>
              </div>
              {ec2Host && ec2Host !== 'localhost' && (
                <p className="mt-3 text-xs text-gray-500">
                  Trying to connect to: {ec2Host}
                </p>
              )}
            </div>
          ) : loading ? (
            <p className="text-yellow-500 mt-2">🔄 Connecting to {ec2Host}...</p>
          ) : (
            <p className="text-green-500 mt-2">✅ Connected to {ec2Host}</p>
          )}
        </div>

        {!error && (
          <div className="w-full bg-gray-900 rounded-lg shadow-2xl border-2 border-green-500 overflow-hidden">
            <iframe
              src={vncUrl}
              className="w-full border-0"
              style={{ height: '70vh', minHeight: '600px' }}
              onLoad={() => setLoading(false)}
              onError={() => {
                setError('Failed to load VNC viewer');
                setLoading(false);
              }}
              title="PersonalOS VNC Viewer"
            />
          </div>
        )}

        <div className="mt-6 text-center text-gray-500 text-sm">
          <p>Built with Rust 🦀 | Powered by AI 🤖 | Hosted on AWS ☁️</p>
          <p className="mt-2">
            <a 
              href="https://github.com/vargasjr-dev/personal-os" 
              target="_blank" 
              rel="noopener noreferrer"
              className="text-green-400 hover:underline"
            >
              View Source on GitHub →
            </a>
          </p>
          {!error && (
            <p className="mt-4 text-xs text-gray-600">
              Direct access: <a href={vncUrl} className="text-green-500 hover:underline" target="_blank" rel="noopener noreferrer">{vncUrl}</a>
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
