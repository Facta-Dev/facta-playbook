// <app> — the Metro configuration.
//
// The app is one package of a workspace: the shared packages live outside
// the app folder, and pnpm links them as symlinks. Metro needs the
// workspace root as a watch folder, otherwise it cannot follow the links
// out of the app directory.
//
// ponytail: this file exists because the workspace layout needs it. Drop it
// when the app is the only package of the repository.

/* global require, module, __dirname */

const path = require("node:path");
const { getDefaultConfig } = require("expo/metro-config");

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, "..");

const config = getDefaultConfig(projectRoot);

// Watch the whole workspace, not only the app folder.
config.watchFolders = [workspaceRoot];

// Resolve the dependencies from the app first, then from the workspace root.
config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, "node_modules"),
  path.resolve(workspaceRoot, "node_modules"),
];

// One copy of react and react-native: a nested duplicate breaks the
// renderer at runtime.
config.resolver.disableHierarchicalLookup = true;

module.exports = config;
