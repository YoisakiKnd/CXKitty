<script>
  /**
   * 扫码登录 dialog.
   *
   * Mirrors the original page: the QR image is shown, the dialog is dismissible,
   * and the session store polls the backend every 2s until the phone confirms.
   * The image comes straight from the backend as a base64 data URI
   * (`start_qr_login`), so no QR library is involved on either side.
   */
  import { Dialog as DialogPrimitive } from 'bits-ui';
  import { Button } from '$lib/components/ui/button/index.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const session = app.session;
</script>

<DialogPrimitive.Root
  open={session.qrOpen}
  onOpenChange={(open) => {
    if (!open) session.closeQr();
  }}
>
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay
      class="data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50"
    />
    <DialogPrimitive.Content
      class="bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-1/2 left-1/2 z-50 w-[calc(100%-2rem)] max-w-sm -translate-x-1/2 -translate-y-1/2 space-y-4 rounded-lg border p-5 shadow-lg"
    >
      <DialogPrimitive.Title class="text-sm font-semibold">扫码登录</DialogPrimitive.Title>
      <DialogPrimitive.Description class="text-muted-foreground text-xs">
        使用学习通 App 扫描二维码，扫描后本窗口会自动关闭。
      </DialogPrimitive.Description>

      <div class="flex flex-col items-center gap-3">
        {#if session.qrImage}
          <img
            src={session.qrImage}
            alt="学习通登录二维码"
            width="220"
            height="220"
            class="bg-muted h-[220px] w-[220px] rounded-md border object-contain"
          />
        {:else}
          <div
            class="bg-muted text-muted-foreground flex h-[220px] w-[220px] items-center justify-center rounded-md border text-xs"
            role="status"
          >
            正在获取二维码…
          </div>
        {/if}
        <p class="text-muted-foreground text-[0.6875rem]" role="status">等待扫码确认…</p>
      </div>

      <div class="flex justify-end">
        <Button variant="outline" onclick={() => session.closeQr()}>关闭</Button>
      </div>
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>
