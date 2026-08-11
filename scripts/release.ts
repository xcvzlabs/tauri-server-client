import * as v from 'valibot';

type ReleaseType = v.InferOutput<typeof releaseTypeSchema>;
type Version = v.InferOutput<typeof versionSchema>;

const releaseTypeSchema = v.picklist(['patch', 'minor', 'major']);
const versionSchema = v.pipe(
  v.string(),
  v.regex(/^\d+\.\d+\.\d+$/, 'Invalid semantic version'),
  v.transform((version) => {
    const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);

    if (!match) {
      const message = 'unreachable: regex already validated shape';
      throw new Error(message);
    }

    const [, major, minor, patch] = match;

    return {
      major: Number(major),
      minor: Number(minor),
      patch: Number(patch),
    };
  }),
);
const packageJsonSchema = v.object({ version: versionSchema });

function run(cmd: string, args: string[]): void {
  const result = Bun.spawnSync([cmd, ...args], { stdio: ['inherit', 'inherit', 'inherit'] });

  if (!result.success) {
    const message = `[RELEASE_COMMAND_FAILED] ${cmd} ${args.join(' ')} exited with code ${result.exitCode}`;
    throw new Error(message);
  }
}

function computeNextVersion(version: Version, type: ReleaseType): string {
  const { major, minor, patch } = version;
  if (type === 'patch') return `${major}.${minor}.${patch + 1}`;
  if (type === 'minor') return `${major}.${minor + 1}.0`;
  if (major === 0) return minor === 0 ? `${major}.${minor + 1}.0` : '1.0.0';
  return `${major + 1}.0.0`;
}

const releaseType = v.parse(releaseTypeSchema, process.argv[2]);
const packageJson = v.parse(packageJsonSchema, await Bun.file('package.json').json());
const nextVersion = computeNextVersion(packageJson.version, releaseType);
const tag = `v${nextVersion}`;

run('bunx', ['changelogen', '-r', nextVersion, '--bump']);
run('bun', ['run', 'scripts/sync-tauri-version.ts']);
run('git', [
  'add',
  'package.json',
  'CHANGELOG.md',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
  'src-tauri/tauri.conf.json',
]);
run('git', ['commit', '-m', `chore(release): ${tag}`]);
run('git', ['tag', '-a', tag, '-m', tag]);
run('git', ['push', '--follow-tags']);
