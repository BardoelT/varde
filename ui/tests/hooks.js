// Gives every app a module registry of its own. `import('.../main.js?app=3')` loads main.js and,
// through this hook, every module it imports as `?app=3` too, so state in modules such as the
// language of i18n.js never leaks from one test into the next.
import { registerHooks } from 'node:module';
import { fileURLToPath } from 'node:url';

const uiRoot = fileURLToPath(new URL('..', import.meta.url));
const testsRoot = fileURLToPath(new URL('.', import.meta.url));

registerHooks({
  resolve(specifier, context, nextResolve) {
    const resolved = nextResolve(specifier, context);
    const app = context.parentURL && new URL(context.parentURL).searchParams.get('app');
    if (!app || !resolved.url.startsWith('file:')) return resolved;
    const path = fileURLToPath(resolved.url);
    if (!path.startsWith(uiRoot) || path.startsWith(testsRoot) || new URL(resolved.url).searchParams.has('app')) return resolved;
    return { ...resolved, url: `${resolved.url}?app=${app}` };
  },
});
