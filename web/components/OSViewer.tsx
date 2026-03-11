'use client';

import { useState } from 'react';

export default function OSViewer({ ec2Host }: { ec2Host: string }) {
  const [loading, setLoading] = useState(true);

  const vncUrl = `http://${ec2Host}:6080/vnc.html?autoconnect=true&resize=scale`;

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-black p-4">
      <div className="w-full max-w-6xl">
        <div className="mb-6 text-center">
          <h1 className="text-4xl font-bold text-green-400 mb-2">
            PersonalOS - Live Demo ⚔️
          </h1>
          <p className="text-gray-400">
            Assistant-Native Operating System running on AWS EC2
          </p>
          <p className="text-yellow-500 mt-2">
            {loading ? '🔄 Loading VNC viewer...' : `✅ Connected to ${ec2Host}`}
          </p>
        </div>

        <div className="w-full bg-gray-900 rounded-lg shadow-2xl border-2 border-green-500 overflow-hidden">
          <iframe
            src={vncUrl}
            className="w-full border-0"
            style={{ height: '70vh', minHeight: '600px' }}
            onLoad={() => setLoading(false)}
            title="PersonalOS VNC Viewer"
          />
        </div>

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
          <p className="mt-4 text-xs text-gray-600">
            Direct access: <a href={vncUrl} className="text-green-500 hover:underline" target="_blank" rel="noopener noreferrer">{vncUrl}</a>
          </p>
        </div>
      </div>
    </div>
  );
}
