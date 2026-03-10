'use client';

import { useEffect, useRef, useState } from 'react';

export default function OSViewer({ ec2Host }: { ec2Host: string }) {
  const canvasRef = useRef<HTMLDivElement>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;

    let rfb: any;

    const connectVNC = async () => {
      try {
        // Dynamically import noVNC (client-side only)
        const RFB = (await import('@novnc/novnc/core/rfb')).default;

        const url = `ws://${ec2Host}:6080/websockify`;
        
        rfb = new RFB(canvasRef.current!, url, {
          credentials: { password: '' },
        });

        rfb.scaleViewport = true;
        rfb.resizeSession = true;

        rfb.addEventListener('connect', () => {
          setConnected(true);
          setError(null);
        });

        rfb.addEventListener('disconnect', () => {
          setConnected(false);
        });

        rfb.addEventListener('securityfailure', (e: any) => {
          setError('Security failure: ' + e.detail.reason);
        });

      } catch (err) {
        setError('Failed to connect: ' + (err as Error).message);
      }
    };

    connectVNC();

    return () => {
      if (rfb) {
        rfb.disconnect();
      }
    };
  }, [ec2Host]);

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
          {connected && (
            <p className="text-green-500 mt-2">✅ Connected to {ec2Host}</p>
          )}
          {error && (
            <p className="text-red-500 mt-2">❌ {error}</p>
          )}
          {!connected && !error && (
            <p className="text-yellow-500 mt-2">🔄 Connecting to OS...</p>
          )}
        </div>

        <div 
          ref={canvasRef} 
          className="w-full bg-gray-900 rounded-lg shadow-2xl border-2 border-green-500"
          style={{ minHeight: '600px' }}
        />

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
        </div>
      </div>
    </div>
  );
}
