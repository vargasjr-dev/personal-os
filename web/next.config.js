/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  webpack: (config, { isServer }) => {
    config.resolve.fallback = { 
      ...config.resolve.fallback,
      fs: false,
      net: false,
      tls: false,
    };
    
    // Handle noVNC module resolution
    config.resolve.alias = {
      ...config.resolve.alias,
      '@novnc/novnc': '@novnc/novnc',
    };
    
    return config;
  },
}

module.exports = nextConfig
