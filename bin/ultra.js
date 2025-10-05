#!/usr/bin/env node

const { spawn } = require('child_process');
const { join } = require('path');
const { platform, arch } = require('os');

// Map Node.js platform/arch to package names
const PLATFORMS = {
  'darwin-arm64': 'ultra-bundler-darwin-arm64',
  'darwin-x64': 'ultra-bundler-darwin-x64',
  'linux-x64': 'ultra-bundler-linux-x64',
  'linux-arm64': 'ultra-bundler-linux-arm64',
  'win32-x64': 'ultra-bundler-win32-x64',
};

const BINARY_NAMES = {
  'darwin-arm64': 'ultra',
  'darwin-x64': 'ultra',
  'linux-x64': 'ultra',
  'linux-arm64': 'ultra',
  'win32-x64': 'ultra.exe',
};

function getPlatformInfo() {
  const platformKey = `${platform()}-${arch()}`;
  return {
    platformKey,
    packageName: PLATFORMS[platformKey],
    binaryName: BINARY_NAMES[platformKey],
  };
}

function findBinary() {
  const { platformKey, packageName, binaryName } = getPlatformInfo();

  if (!packageName) {
    console.error(`Unsupported platform: ${platformKey}`);
    console.error('Ultra bundler is currently supported on:');
    console.error('  - macOS (Intel and Apple Silicon)');
    console.error('  - Linux (x64 and ARM64)');
    console.error('  - Windows (x64)');
    process.exit(1);
  }

  try {
    // Try to find the native binary in the platform-specific package
    const binaryPath = require.resolve(`${packageName}/bin/${binaryName}`);
    return binaryPath;
  } catch (error) {
    console.error(`Failed to find Ultra binary for ${platformKey}`);
    console.error('');
    console.error('This may happen if:');
    console.error('  1. The optional dependency was not installed');
    console.error('  2. Your platform is not yet supported');
    console.error('');
    console.error('Try reinstalling: npm install ultra-bundler');
    console.error(`Or manually install: npm install ${packageName}`);
    process.exit(1);
  }
}

function run() {
  const binaryPath = findBinary();
  const args = process.argv.slice(2);

  // Spawn the native binary
  const child = spawn(binaryPath, args, {
    stdio: 'inherit',
    windowsHide: true,
  });

  child.on('error', (error) => {
    console.error('Failed to start Ultra:', error.message);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code || 0);
    }
  });
}

// Handle process signals
process.on('SIGINT', () => process.exit(130));
process.on('SIGTERM', () => process.exit(143));

// Run the binary
run();
