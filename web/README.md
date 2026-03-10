# PersonalOS Web Interface

Next.js website for accessing PersonalOS via browser - desktop or mobile.

---

## Quick Start

### Development

```bash
# Install dependencies
npm install

# Create .env.local
cp .env.example .env.local
# Edit .env.local and set NEXT_PUBLIC_EC2_HOST

# Run dev server
npm run dev

# Open http://localhost:3000
```

### Production Deploy

```bash
# Deploy to Vercel
vercel deploy --prod
```

---

## Environment Variables

Set in Vercel dashboard or `.env.local`:

- `NEXT_PUBLIC_EC2_HOST` - EC2 instance IP or hostname (e.g., `ec2-xx-xx-xx-xx.compute-1.amazonaws.com`)

---

## Tech Stack

- **Next.js 14** - React framework
- **TypeScript** - Type safety
- **Tailwind CSS** - Styling
- **noVNC** - Browser VNC client
- **WebSocket** - Real-time connection to EC2

---

## How It Works

1. User opens website on any device (phone, tablet, desktop)
2. React component initializes noVNC client
3. WebSocket connects to EC2 instance on port 6080
4. QEMU running on EC2 streams display via VNC
5. User interacts with OS in real-time

---

## Mobile Optimization

- Touch events mapped to mouse clicks
- Virtual keyboard support (when OS implements input)
- Responsive canvas scaling
- Low-latency WebSocket connection

---

## Deployment

See [DEPLOYMENT.md](../deployment/DEPLOYMENT.md) for full instructions.

**Quick:** `vercel deploy --prod`

---

## Customization

### Change Colors

Edit `tailwind.config.ts`:
```ts
colors: {
  'os-green': '#22c55e',  // Change to your brand color
}
```

### Add Features

- Authentication (protect demo access)
- Multiple OS instances (dropdown selector)
- Performance metrics (latency, FPS)
- Screenshot capture
- Session recording

---

Built with ❤️ for the future of computing.
