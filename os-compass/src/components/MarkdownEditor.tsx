import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import EasyMDE from "easymde";
import "easymde/dist/easymde.min.css";

interface MarkdownEditorProps {
  initialValue?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  minHeight?: string;
}

export function MarkdownEditor({
  initialValue = "",
  onChange,
  placeholder,
  minHeight = "300px",
}: MarkdownEditorProps) {
  const { t } = useTranslation();
  const resolvedPlaceholder = placeholder ?? t("markdownEditor.placeholder");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const editorRef = useRef<EasyMDE | null>(null);

  useEffect(() => {
    if (textareaRef.current && !editorRef.current) {
      editorRef.current = new EasyMDE({
        element: textareaRef.current,
        initialValue,
        placeholder: resolvedPlaceholder,
        spellChecker: false,
        autofocus: false,
        status: false,
        minHeight,
        toolbar: [
          "bold", "italic", "heading", "|",
          "code", "quote", "|",
          "unordered-list", "ordered-list", "|",
          "link", "image", "|",
          "preview", "side-by-side", "|",
          "fullscreen"
        ],
      });

      if (onChange) {
        editorRef.current.codemirror.on("change", () => {
          onChange(editorRef.current!.value());
        });
      }
    }

    return () => {
      if (editorRef.current) {
        editorRef.current.toTextArea();
        editorRef.current = null;
      }
    };
  }, []);

  useEffect(() => {
    if (editorRef.current && initialValue && editorRef.current.value() !== initialValue) {
      editorRef.current.value(initialValue);
    }
  }, [initialValue]);

  return (
    <div className="border rounded-lg overflow-hidden">
      <div className="bg-gray-100 px-4 py-2 border-b flex items-center justify-between">
        <span className="text-sm text-gray-600">{t("markdownEditor.readmeLabel")}</span>
        <span className="text-xs text-gray-400">
          {t("markdownEditor.previewTip")}
        </span>
      </div>
      <div className="markdown-editor-wrapper">
        <textarea ref={textareaRef} />
      </div>
      <style>{`
        .markdown-editor-wrapper .EasyMDEContainer .CodeMirror {
          border: none;
          border-radius: 0;
          min-height: ${minHeight};
        }
        .markdown-editor-wrapper .EasyMDEContainer .editor-toolbar {
          border: none;
          border-bottom: 1px solid #e5e7eb;
          background: #f9fafb;
        }
        .markdown-editor-wrapper .EasyMDEContainer .editor-toolbar button {
          color: #6b7280;
        }
        .markdown-editor-wrapper .EasyMDEContainer .editor-toolbar button:hover,
        .markdown-editor-wrapper .EasyMDEContainer .editor-toolbar button.active {
          background: #e5e7eb;
          color: #374151;
        }
        .markdown-editor-wrapper .EasyMDEContainer .editor-preview {
          background: white;
          padding: 1rem;
          min-height: ${minHeight};
        }
        .markdown-editor-wrapper .EasyMDEContainer .editor-preview-side {
          background: white;
          padding: 1rem;
        }
      `}</style>
    </div>
  );
}
