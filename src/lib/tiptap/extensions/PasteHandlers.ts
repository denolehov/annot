import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';

/**
 * Determine if pasted text should be collapsed into a chip.
 * Focus on screen real estate - chip when content would visually dominate.
 */
export function shouldChip(text: string): boolean {
  if (!text) return false;

  const lines = text.split('\n');
  const lineCount = lines.length;
  const charCount = text.length;

  // Vertical sprawl: takes up too much height (6+ lines)
  if (lineCount >= 6) return true;

  // Horizontal sprawl: massive single/double line (minified, tokens, URLs)
  if (lineCount <= 2 && charCount >= 400) return true;

  return false;
}

/**
 * ImagePasteHandler extension - intercepts paste events and inserts MediaChip nodes for images.
 * Only active when image paste is allowed (MCP content mode).
 */
export interface ImagePasteHandlerOptions {
  allowsImagePaste: boolean;
  onPasteBlocked?: () => void;
}

export const ImagePasteHandler = Extension.create<ImagePasteHandlerOptions>({
  name: 'imagePasteHandler',

  addOptions() {
    return {
      allowsImagePaste: false,
      onPasteBlocked: undefined,
    };
  },

  addStorage() {
    return {
      allowsImagePaste: this.options.allowsImagePaste,
    };
  },

  addProseMirrorPlugins() {
    const extension = this;
    const editor = this.editor;

    return [
      new Plugin({
        key: new PluginKey('imagePasteHandler'),
        props: {
          handlePaste(view, event) {
            const items = event.clipboardData?.items;
            if (!items) return false;

            // Find image in clipboard
            let imageFile: File | null = null;
            for (const item of Array.from(items)) {
              if (item.type.startsWith('image/')) {
                imageFile = item.getAsFile();
                break;
              }
            }

            if (!imageFile) return false;

            // Check allowsImagePaste from storage
            const { allowsImagePaste } = extension.storage;
            const { onPasteBlocked } = extension.options;

            // Block paste if not allowed
            if (!allowsImagePaste) {
              onPasteBlocked?.();
              return true; // Consume the event
            }

            // Convert to base64 and insert MediaChip
            const reader = new FileReader();
            reader.onloadend = () => {
              const dataUrl = reader.result as string;
              editor
                .chain()
                .focus()
                .insertContent([
                  {
                    type: 'mediaChip',
                    attrs: {
                      image: dataUrl,
                      mimeType: imageFile!.type,
                    },
                  },
                  { type: 'text', text: ' ' },
                ])
                .run();
            };
            reader.readAsDataURL(imageFile);

            return true; // Consume the event
          },
        },
      }),
    ];
  },
});

/**
 * TextPasteHandler extension - intercepts text paste events and inserts PasteChip nodes
 * for large text content that would visually dominate the editor.
 */
export const TextPasteHandler = Extension.create({
  name: 'textPasteHandler',

  addProseMirrorPlugins() {
    const editor = this.editor;

    return [
      new Plugin({
        key: new PluginKey('textPasteHandler'),
        props: {
          handlePaste(view, event) {
            const clipboardData = event.clipboardData;
            if (!clipboardData) return false;

            // Only handle if there's no image (let ImagePasteHandler handle those)
            const hasImage = Array.from(clipboardData.items).some((item) =>
              item.type.startsWith('image/')
            );
            if (hasImage) return false;

            // Get plain text from clipboard
            const text = clipboardData.getData('text/plain');
            if (!text) return false;

            // Check if this text should be chipped
            if (!shouldChip(text)) return false;

            // Insert PasteChip instead of raw text
            const lineCount = text.split('\n').length;
            editor
              .chain()
              .focus()
              .insertContent([
                {
                  type: 'pasteChip',
                  attrs: {
                    content: text,
                    lineCount,
                  },
                },
                { type: 'text', text: ' ' },
              ])
              .run();

            return true; // Consume the event
          },
        },
      }),
    ];
  },
});
