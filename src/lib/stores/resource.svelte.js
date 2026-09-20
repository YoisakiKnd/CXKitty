/**
 * Async resource helper.
 *
 * Gives every piece of remotely-loaded data the same four states
 * (`idle` → `loading` → `ready` / `empty` / `error`) plus an explicit `retry()`,
 * so no view has to invent its own loading and failure handling.
 *
 * Runes live in a `.svelte.js` module, which is the supported place for
 * reactive state that is shared across components.
 */

/**
 * @template T
 * @param {() => Promise<T>} loader
 * @param {{ isEmpty?: (data: T) => boolean, initial?: T | null }} [options]
 */
export function createResource(loader, options = {}) {
  const isEmpty = options.isEmpty ?? ((data) => data == null);
  // Annotated so `??` does not narrow `T` down to `NonNullable<T>`: `data` must
  // stay able to hold a legitimately nullish `T` returned by the loader.
  /** @type {T | null} */
  const initial = options.initial ?? null;

  let status = $state('idle'); // idle | loading | ready | empty | error
  let data = $state(initial);
  let error = $state('');
  let settled = $state(false); // at least one attempt finished (success or failure)
  let inFlight = 0;

  async function load() {
    const ticket = ++inFlight;
    status = 'loading';
    error = '';
    try {
      const result = await loader();
      if (ticket !== inFlight) return data; // superseded by a newer request
      data = result;
      settled = true;
      status = isEmpty(result) ? 'empty' : 'ready';
      return result;
    } catch (e) {
      if (ticket !== inFlight) return data;
      error = e instanceof Error ? e.message : String(e);
      settled = true;
      status = 'error';
      return null;
    }
  }

  /** Clear to the pristine state (used when the account/session changes). */
  function reset() {
    inFlight++;
    status = 'idle';
    data = initial;
    error = '';
    settled = false;
  }

  /** Mark as empty without a round trip (e.g. "no course selected yet"). */
  function setIdle() {
    inFlight++;
    status = 'idle';
    data = initial;
    error = '';
  }

  return {
    get status() {
      return status;
    },
    get data() {
      return data;
    },
    get error() {
      return error;
    },
    get settled() {
      return settled;
    },
    get loading() {
      return status === 'loading';
    },
    get failed() {
      return status === 'error';
    },
    get isEmpty() {
      return status === 'empty';
    },
    get isReady() {
      return status === 'ready';
    },
    load,
    retry: load,
    reset,
    setIdle,
  };
}
