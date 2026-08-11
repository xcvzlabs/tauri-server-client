import * as v from 'valibot';

const tauriDir = 'src-tauri';
const cargoTomlPath = `${tauriDir}/Cargo.toml`;
const cargoLockPath = `${tauriDir}/Cargo.lock`;
const tauriConfigPath = `${tauriDir}/tauri.conf.json`;

const versionSchema = v.pipe(v.string(), v.regex(/^\d+\.\d+\.\d+$/, 'Invalid semantic version'));
const packageJsonSchema = v.object({ version: versionSchema });

function escapeRegExp(value: string): string {
  return value.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
}

async function writeJsonVersion(path: string, version: string): Promise<void> {
  const source = await Bun.file(path).text();
  const updated = source.replace(/^(\s*"version":\s*")[^"]+(")/m, `$1${version}$2`);

  if (updated === source && !source.includes(`"version": "${version}"`)) {
    const message = `[SYNC_TAURI_VERSION_TARGET_MISSING] No version field found in ${path}`;
    throw new Error(message);
  }

  await Bun.write(path, updated);
}

const packageJson = v.parse(packageJsonSchema, await Bun.file('package.json').json());
const { version } = packageJson;

await writeJsonVersion(tauriConfigPath, version);

const cargoToml = await Bun.file(cargoTomlPath).text();
const packageNameMatch = /^name = "([^"]+)"$/m.exec(cargoToml);

if (!packageNameMatch?.[1]) {
  const message = `[SYNC_TAURI_VERSION_CARGO_NAME_UNRESOLVED] No package name in ${cargoTomlPath}`;
  throw new Error(message);
}

await Bun.write(cargoTomlPath, cargoToml.replace(/^version = "[^"]+"$/m, `version = "${version}"`));

const cargoPackageName = packageNameMatch[1];
const cargoLock = await Bun.file(cargoLockPath).text();
const pattern = `(name = "${escapeRegExp(cargoPackageName)}"\\nversion = ")[^"]+(")`;
const cargoLockPattern = new RegExp(pattern);

if (!cargoLockPattern.test(cargoLock)) {
  const message = `[SYNC_TAURI_VERSION_CARGO_LOCK_UNRESOLVED] Package ${cargoPackageName} not found in ${cargoLockPath}`;
  throw new Error(message);
}

await Bun.write(cargoLockPath, cargoLock.replace(cargoLockPattern, `$1${version}$2`));
