/**
 * Shared `tailwind-variants` recipes.
 *
 * Every badge / status chip in the app resolves through these, which keeps
 * colour, spacing, radius and typography identical everywhere instead of each
 * component inventing its own utility soup.
 */
import { tv } from 'tailwind-variants';

/** Small status chip (badges in tables, task state, chapter state). */
export const badgeTone = tv({
  base: 'inline-flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 text-[0.6875rem] leading-4 font-medium whitespace-nowrap',
  variants: {
    tone: {
      neutral: 'border-transparent bg-secondary text-secondary-foreground',
      muted: 'border-border bg-muted text-muted-foreground',
      ok: 'border-transparent bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300',
      info: 'border-transparent bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-300',
      warn: 'border-transparent bg-amber-100 text-amber-900 dark:bg-amber-950 dark:text-amber-300',
      danger: 'border-transparent bg-rose-100 text-rose-900 dark:bg-rose-950 dark:text-rose-300',
    },
  },
  defaultVariants: { tone: 'neutral' },
});

/** Muted helper text. */
export const mutedText = tv({
  base: 'text-muted-foreground text-xs leading-relaxed',
});

/** Two-column key/value grid used by the task progress panel. */
export const statusGrid = tv({
  base: 'grid w-full grid-cols-1 gap-x-4 gap-y-3 sm:grid-cols-2 lg:grid-cols-3',
});

/** Progress track + fill for the two progress bars. */
export const progressTrack = tv({
  base: 'h-2 w-full overflow-hidden rounded-full bg-muted',
});

export const progressFill = tv({
  base: 'h-full rounded-full transition-[width] duration-300 ease-out',
  variants: {
    tone: {
      primary: 'bg-primary',
      wait: 'bg-amber-500',
    },
  },
  defaultVariants: { tone: 'primary' },
});
