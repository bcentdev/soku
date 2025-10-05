#!/usr/bin/env node

const { platform, arch } = require('os');
const { existsSync } = require('fs');
const { join } = require('path');

const PLATFORMS = {
  'darwin-arm64': '@ultra-bundler/darwin-arm64',
  'darwin-x64': '@ultra-bundler/darwin-x64',
  'linux-x64': '@ultra-bundler/linux-x64',
  'linux-arm64': '@ultra-bundler/linux-arm64',
  'win32-x64': '@ultra-bundler/win32-x64',
};

function checkInstallation() {
  const platformKey = `${platform()}-${arch()}`;
  const packageName = PLATFORMS[platformKey];

  if (!packageName) {
    console.warn(`⚠️  Ultra bundler: Platform ${platformKey} is not officially supported.`);
    console.warn('You may need to build from source: https://github.com/bcentdev/ultra');
    return;
  }

  try {
    // Check if the platform-specific package is installed
    require.resolve(packageName);
    console.log(`✓ Ultra bundler installed successfully for ${platformKey}`);
  } catch (error) {
    console.warn(`⚠️  Optional dependency ${packageName} was not installed.`);
    console.warn('This is usually fine - npm will install it automatically.');
    console.warn('If Ultra fails to run, try: npm install ' + packageName);
  }
}

// Only run installation check if not in CI
if (!process.env.CI) {
  checkInstallation();
}
