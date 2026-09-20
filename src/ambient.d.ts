/**
 * Ambient module declarations for the JS typecheck (`npm run typecheck`).
 *
 * Vite resolves `.svelte` and `.css` imports, but `tsc` does not know those
 * extensions. Declaring them here keeps the check honest — it still type-checks
 * every `.js` module in `src/` — without pulling in `svelte-check`/`svelte2tsx`,
 * which are not repository dependencies.
 */

declare module '*.svelte' {
  /** A Svelte 5 component, mounted with `mount()` or rendered in a template. */
  const component: import('svelte').Component;
  export default component;
}

declare module '*.css';
