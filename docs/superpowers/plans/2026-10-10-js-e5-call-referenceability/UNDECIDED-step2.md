# Every UNDECIDED union removal

11 targets. No WRONG yield credit. Each line names the own-file binding and the reason the oracle cannot decide.

1. X `packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:85` `addAssistantMessage` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:22`; ordinary, b_only; local binding: addAssistantMessage; oracle: signature_matches_without_value_binding_identity.
2. X `packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:84` `addUserMessage` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:11`; ordinary, b_only; local binding: addUserMessage; oracle: signature_matches_without_value_binding_identity.
3. X `packages/excalidraw/components/LibraryMenuHeaderContent.tsx:308` `clearLibraryCache` → `packages/excalidraw/hooks/useLibraryItemSvg.ts:90`; ordinary, b_only; local binding: clearLibraryCache; oracle: signature_matches_without_value_binding_identity.
4. X `packages/excalidraw/components/LibraryMenuHeaderContent.tsx:301` `deleteItemsFromLibraryCache` → `packages/excalidraw/hooks/useLibraryItemSvg.ts:92`; ordinary, b_only; local binding: deleteItemsFromLibraryCache; oracle: signature_matches_without_value_binding_identity.
5. X `packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:186` `setAssistantError` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:42`; ordinary, b_only; local binding: setAssistantError; oracle: signature_matches_without_value_binding_identity.
6. X `packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:201` `setAssistantError` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:42`; ordinary, b_only; local binding: setAssistantError; oracle: signature_matches_without_value_binding_identity.
7. X `packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:208` `setAssistantError` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:42`; ordinary, b_only; local binding: setAssistantError; oracle: signature_matches_without_value_binding_identity.
8. X `packages/excalidraw/components/TTDDialog/TextToDiagram.tsx:157` `setLastRetryAttempt` → `packages/excalidraw/components/TTDDialog/Chat/useChatAgent.ts:34`; ordinary, b_only; local binding: setLastRetryAttempt; oracle: signature_matches_without_value_binding_identity.
9. X `packages/excalidraw/components/ShareableLinkDialog.tsx:35` `t` → `packages/excalidraw/i18n.ts:127`; ordinary, b_only; local binding: t; oracle: signature_matches_without_value_binding_identity.
10. X `packages/excalidraw/components/main-menu/DefaultItems.tsx:79` `t` → `packages/excalidraw/i18n.ts:127`; ordinary, b_only; local binding: t; oracle: signature_matches_without_value_binding_identity.
11. X `packages/excalidraw/components/main-menu/DefaultItems.tsx:80` `t` → `packages/excalidraw/i18n.ts:127`; ordinary, b_only; local binding: t; oracle: signature_matches_without_value_binding_identity.

## Public samples

`X / packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:85`

```tsx
      addUserMessage(prompt);
      addAssistantMessage();
    } else {
```

`X / packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:84`

```tsx
    if (!isRepairFlow) {
      addUserMessage(prompt);
      addAssistantMessage();
```

`X / packages/excalidraw/components/LibraryMenuHeaderContent.tsx:308`

```tsx
    library.resetLibrary();
    clearLibraryCache();
  };
```

`X / packages/excalidraw/components/LibraryMenuHeaderContent.tsx:301`

```tsx

    deleteItemsFromLibraryCache(selectedItems);

```

`X / packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:186`

```tsx
        if (error.status !== 429) {
          setAssistantError(_error.message, "network");
        }
```

`X / packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:201`

```tsx
        );
        setAssistantError(_error.message, "parse");
        setError(_error);
```

`X / packages/excalidraw/components/TTDDialog/hooks/useTextGeneration.ts:208`

```tsx
      );
      setAssistantError(_error.message, "other");
      setError(_error);
```

`X / packages/excalidraw/components/TTDDialog/TextToDiagram.tsx:157`

```tsx
      ) {
        setLastRetryAttempt();
        await onGenerate({
```

`X / packages/excalidraw/components/ShareableLinkDialog.tsx:35`

```tsx
    } catch (e) {
      setErrorMessage(t("errors.copyToSystemClipboardFailed"));
    }
```

`X / packages/excalidraw/components/main-menu/DefaultItems.tsx:79`

```tsx
      (await openConfirmModal({
        title: t("overwriteConfirm.modal.loadFromFile.title"),
        actionLabel: t("overwriteConfirm.modal.loadFromFile.button"),
```

