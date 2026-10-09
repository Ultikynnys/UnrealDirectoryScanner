/* Points git at the committed hooks directory instead of the untracked .git/hooks, so the
   pre-commit autotests travel with the repository. Run by `npm install` through the prepare
   script, and harmless anywhere git is not in play: a checkout by zip, or a copy of the
   sources, should still install. */
import { execFileSync } from 'node:child_process';
import { chmodSync } from 'node:fs';

try {
  execFileSync('git', ['config', 'core.hooksPath', '.githooks'], { stdio: 'inherit' });
  // git for Windows does not record the executable bit, so set it wherever it matters.
  if (process.platform !== 'win32') chmodSync('.githooks/pre-commit', 0o755);
  console.log('pre-commit autotests wired up from .githooks');
} catch {
  console.log('not a git checkout; skipping the pre-commit hook setup');
}
