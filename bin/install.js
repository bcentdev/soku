#!/usr/bin/env node

const { platform, arch } = require('os');
const { existsSync } = require('fs');
const { join } = require('path');

const PLATFORMS = {
  'darwin-arm64': 'soku-darwin-arm64',
  'darwin-x64': 'soku-darwin-x64',
  'linux-x64': 'soku-linux-x64',
  'linux-arm64': 'soku-linux-arm64',
  'win32-x64': 'soku-win32-x64',
};

function checkInstallation() {
  const platformKey = `${platform()}-${arch()}`;
  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    console.warn(`⚠️  Soku bundler: Platform ${platformKey} is not officially supported.`);
    console.warn('You may need to build from source: https://github.com/bcentdev/soku');
    return;
  }

  try {
    // Check if the platform-specific package is installed
    require.resolve(packageName);
    console.log(`✓ Soku (速) bundler installed successfully for ${platformKey}`);
  } catch (error) {
    console.warn(`⚠️  Optional dependency ${packageName} was not installed.`);
    console.warn('This is usually fine - npm will install it automatically.');
    console.warn('If Soku fails to run, try: npm install ' + packageName);
  }
}

// Only run installation check if not in CI
if (!process.env.CI) {
  checkInstallation();
}
