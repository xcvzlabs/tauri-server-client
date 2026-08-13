import * as v from 'valibot';

const tauriDir = 'src-tauri';
const cargoTomlPath = `${tauriDir}/Cargo.toml`;
const cargoLockPath = `${tauriDir}/Cargo.lock`;
const tauriConfigPath = `${tauriDir}/tauri.conf.json`;

const serverDir = 'server';
const serverCargoTomlPath = `${serverDir}/Cargo.toml`;
const serverCargoLockPath = `${serverDir}/Cargo.lock`;

const versionSchema = v.pipe(v.string(), v.regex(/^\d+\.\d+\.\d+$/, 'Invalid semantic version'));
const packageJsonSchema = v.object({ version: versionSchema });

function escapeRegExp(value: string): string {
  return value.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
}

async function writeJsonVersion(path: string, version: string): Promise<void> {
  const source = await Bun.file(path).text();
  const updated = source.replace(/^(\s*"version":\s*")[^"]+(")/m, `$1${version}$2`);

  if (updated === source && !source.includes(`"version": "${version}"`)) {
    const message = `[SYNC_VERSIONS_TARGET_MISSING] No version field found in ${path}`;
    throw new Error(message);
  }

  await Bun.write(path, updated);
}

async function syncCargoPackageVersion(
  packageCargoTomlPath: string,
  packageCargoLockPath: string,
  version: string,
): Promise<void> {
  const cargoToml = await Bun.file(packageCargoTomlPath).text();
  const packageNameMatch = /^name = "([^"]+)"$/m.exec(cargoToml);

  if (!packageNameMatch?.[1]) {
    const message = `[SYNC_VERSIONS_CARGO_NAME_UNRESOLVED] No package name in ${packageCargoTomlPath}`;
    throw new Error(message);
  }

  await Bun.write(
    packageCargoTomlPath,
    cargoToml.replace(/^version = "[^"]+"$/m, `version = "${version}"`),
  );

  const cargoPackageName = packageNameMatch[1];
  const cargoLock = await Bun.file(packageCargoLockPath).text();
  const pattern = `(name = "${escapeRegExp(cargoPackageName)}"\\nversion = ")[^"]+(")`;
  const cargoLockPattern = new RegExp(pattern);

  if (!cargoLockPattern.test(cargoLock)) {
    const message = `[SYNC_VERSIONS_CARGO_LOCK_UNRESOLVED] Package ${cargoPackageName} not found in ${packageCargoLockPath}`;
    throw new Error(message);
  }

  await Bun.write(packageCargoLockPath, cargoLock.replace(cargoLockPattern, `$1${version}$2`));
}

const packageJson = v.parse(packageJsonSchema, await Bun.file('package.json').json());
const { version } = packageJson;

await writeJsonVersion(tauriConfigPath, version);
await syncCargoPackageVersion(cargoTomlPath, cargoLockPath, version);
await syncCargoPackageVersion(serverCargoTomlPath, serverCargoLockPath, version);
