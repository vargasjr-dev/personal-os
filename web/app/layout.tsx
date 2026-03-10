import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'PersonalOS - Assistant-Native Operating System',
  description: 'Live demo of PersonalOS - an operating system built from scratch in Rust with AI assistants as first-class citizens.',
  keywords: ['operating system', 'rust', 'ai', 'assistant-native', 'llm'],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
