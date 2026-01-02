import { useState } from "react";
import {
  Wand2,
  Plus,
  X,
  GripVertical,
  ChevronDown,
  ChevronUp,
  Pencil,
  ToggleLeft,
  ToggleRight,
  RotateCcw,
  FileType,
  Regex,
  FolderOpen,
  FileText,
  Sparkles,
} from "lucide-react";
import { cn } from "../../lib/utils";
import {
  useDownloadsWatcherStore,
  type CustomRenameRule,
} from "../../stores/downloads-watcher-store";

const MATCH_TYPE_OPTIONS = [
  { value: "extension", label: "File Extension", icon: FileType, example: ".pdf, .jpg" },
  { value: "pattern", label: "Filename Pattern", icon: Regex, example: "Screenshot*, *.tmp" },
  { value: "folder", label: "Source Folder", icon: FolderOpen, example: "/Downloads" },
  { value: "content", label: "File Content", icon: FileText, example: "invoice, receipt" },
] as const;

const TRANSFORM_TYPE_OPTIONS = [
  { value: "prefix", label: "Add Prefix", example: "doc-filename.pdf" },
  { value: "suffix", label: "Add Suffix", example: "filename-backup.pdf" },
  { value: "replace", label: "Find & Replace", example: "old → new" },
  { value: "template", label: "Template", example: "{type}-{date}.{ext}" },
  { value: "ai-prompt", label: "AI Custom Prompt", example: "Custom AI instructions" },
] as const;

interface CustomRulesEditorProps {
  compact?: boolean;
}

export function CustomRulesEditor({ compact = false }: CustomRulesEditorProps) {
  const {
    customRules,
    rulesEnabled,
    addRule,
    updateRule,
    removeRule,
    toggleRuleEnabled,
    setRulesEnabled,
    resetToDefaultRules,
  } = useDownloadsWatcherStore();

  const [expanded, setExpanded] = useState(!compact);
  const [editingRule, setEditingRule] = useState<string | null>(null);
  const [isAddingRule, setIsAddingRule] = useState(false);

  const enabledRulesCount = customRules.filter((r) => r.enabled).length;

  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100"
        >
          <Wand2 size={16} />
          <span>Custom Rename Rules</span>
          <span className="px-1.5 py-0.5 text-xs rounded-full bg-gray-200 dark:bg-gray-700">
            {enabledRulesCount}/{customRules.length}
          </span>
          {expanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </button>

        {/* Global toggle */}
        <button
          onClick={() => setRulesEnabled(!rulesEnabled)}
          className={cn(
            "flex items-center gap-1 text-xs px-2 py-1 rounded transition-colors",
            rulesEnabled
              ? "text-green-600 bg-green-50 dark:bg-green-900/20"
              : "text-gray-400 bg-gray-100 dark:bg-gray-800"
          )}
        >
          {rulesEnabled ? <ToggleRight size={14} /> : <ToggleLeft size={14} />}
          {rulesEnabled ? "Active" : "Disabled"}
        </button>
      </div>

      {expanded && (
        <div className="space-y-2">
          {/* Rules list */}
          {customRules.length > 0 ? (
            <div className="space-y-1">
              {customRules.map((rule) => (
                <RuleItem
                  key={rule.id}
                  rule={rule}
                  isEditing={editingRule === rule.id}
                  onEdit={() =>
                    setEditingRule(editingRule === rule.id ? null : rule.id)
                  }
                  onToggle={() => toggleRuleEnabled(rule.id)}
                  onRemove={() => removeRule(rule.id)}
                  onUpdate={(updates) => updateRule(rule.id, updates)}
                  globalEnabled={rulesEnabled}
                />
              ))}
            </div>
          ) : (
            <div className="p-4 text-center text-sm text-gray-500 dark:text-gray-400 border-2 border-dashed border-gray-200 dark:border-gray-700 rounded-lg">
              <Wand2 size={24} className="mx-auto mb-2 opacity-50" />
              <p>No custom rules</p>
              <p className="text-xs mt-1">Add rules to customize how files are renamed</p>
            </div>
          )}

          {/* Add rule section */}
          {isAddingRule ? (
            <AddRuleForm
              onAdd={(rule) => {
                addRule(rule);
                setIsAddingRule(false);
              }}
              onCancel={() => setIsAddingRule(false)}
            />
          ) : (
            <div className="flex items-center gap-2">
              <button
                onClick={() => setIsAddingRule(true)}
                className="flex-1 flex items-center justify-center gap-2 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-orange-500 dark:hover:text-orange-400 transition-colors border border-dashed border-gray-300 dark:border-gray-600 rounded-lg hover:border-orange-400"
              >
                <Plus size={16} />
                Add custom rule
              </button>

              {customRules.length > 0 && (
                <button
                  onClick={() => {
                    if (confirm("Reset to default rules?")) {
                      resetToDefaultRules();
                    }
                  }}
                  className="p-2 text-gray-400 hover:text-orange-500 transition-colors"
                  title="Reset to defaults"
                >
                  <RotateCcw size={16} />
                </button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface RuleItemProps {
  rule: CustomRenameRule;
  isEditing: boolean;
  onEdit: () => void;
  onToggle: () => void;
  onRemove: () => void;
  onUpdate: (updates: Partial<CustomRenameRule>) => void;
  globalEnabled: boolean;
}

function RuleItem({
  rule,
  isEditing,
  onEdit,
  onToggle,
  onRemove,
  onUpdate,
  globalEnabled,
}: RuleItemProps) {
  const isActive = globalEnabled && rule.enabled;
  const matchType = MATCH_TYPE_OPTIONS.find((m) => m.value === rule.matchType);
  const MatchIcon = matchType?.icon || FileType;

  return (
    <div
      className={cn(
        "rounded-lg border transition-colors",
        isActive
          ? "bg-white dark:bg-[#2a2a2a] border-gray-200 dark:border-gray-700"
          : "bg-gray-50 dark:bg-gray-800/50 border-gray-200 dark:border-gray-700 opacity-60"
      )}
    >
      {/* Rule header */}
      <div className="flex items-center gap-2 p-2">
        <GripVertical
          size={14}
          className="text-gray-300 dark:text-gray-600 cursor-grab flex-shrink-0"
        />

        <div className="flex items-center gap-2 flex-1 min-w-0">
          <MatchIcon size={14} className="text-gray-400 flex-shrink-0" />
          <div className="min-w-0">
            <p
              className={cn(
                "text-sm font-medium truncate",
                isActive
                  ? "text-gray-900 dark:text-gray-100"
                  : "text-gray-500 dark:text-gray-400"
              )}
            >
              {rule.name}
            </p>
            <p className="text-xs text-gray-400 truncate">{rule.description}</p>
          </div>
        </div>

        <div className="flex items-center gap-1 flex-shrink-0">
          {rule.transformType === "ai-prompt" && (
            <Sparkles size={12} className="text-purple-500" />
          )}

          <button
            onClick={onEdit}
            className="p-1.5 rounded text-gray-400 hover:text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700"
          >
            <Pencil size={12} />
          </button>

          <button
            onClick={onToggle}
            className={cn(
              "p-1.5 rounded transition-colors",
              rule.enabled
                ? "text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20"
                : "text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
            )}
          >
            {rule.enabled ? <ToggleRight size={14} /> : <ToggleLeft size={14} />}
          </button>

          <button
            onClick={onRemove}
            className="p-1.5 rounded text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            <X size={12} />
          </button>
        </div>
      </div>

      {/* Expanded edit form */}
      {isEditing && (
        <div className="p-3 pt-0 border-t border-gray-100 dark:border-gray-700 space-y-3">
          <div className="grid grid-cols-2 gap-3">
            {/* Match type */}
            <div>
              <label className="block text-xs text-gray-500 mb-1">Match Type</label>
              <select
                value={rule.matchType}
                onChange={(e) =>
                  onUpdate({ matchType: e.target.value as CustomRenameRule["matchType"] })
                }
                className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
              >
                {MATCH_TYPE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Match value */}
            <div>
              <label className="block text-xs text-gray-500 mb-1">
                Match Value
                <span className="text-gray-400 ml-1">({matchType?.example})</span>
              </label>
              <input
                type="text"
                value={rule.matchValue}
                onChange={(e) => onUpdate({ matchValue: e.target.value })}
                placeholder={matchType?.example}
                className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
              />
            </div>

            {/* Transform type */}
            <div>
              <label className="block text-xs text-gray-500 mb-1">Transform</label>
              <select
                value={rule.transformType}
                onChange={(e) =>
                  onUpdate({
                    transformType: e.target.value as CustomRenameRule["transformType"],
                  })
                }
                className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
              >
                {TRANSFORM_TYPE_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>

            {/* Transform value */}
            <div>
              <label className="block text-xs text-gray-500 mb-1">
                {rule.transformType === "ai-prompt" ? "AI Instructions" : "Value"}
              </label>
              {rule.transformType === "ai-prompt" ? (
                <textarea
                  value={rule.transformValue}
                  onChange={(e) => onUpdate({ transformValue: e.target.value })}
                  placeholder="Custom AI instructions for renaming..."
                  rows={2}
                  className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700 resize-none"
                />
              ) : (
                <input
                  type="text"
                  value={rule.transformValue}
                  onChange={(e) => onUpdate({ transformValue: e.target.value })}
                  className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
                />
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

interface AddRuleFormProps {
  onAdd: (rule: Omit<CustomRenameRule, "id">) => void;
  onCancel: () => void;
}

function AddRuleForm({ onAdd, onCancel }: AddRuleFormProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [matchType, setMatchType] = useState<CustomRenameRule["matchType"]>("pattern");
  const [matchValue, setMatchValue] = useState("");
  const [transformType, setTransformType] =
    useState<CustomRenameRule["transformType"]>("ai-prompt");
  const [transformValue, setTransformValue] = useState("");

  const handleSubmit = () => {
    if (!name.trim()) return;

    onAdd({
      name: name.trim(),
      description: description.trim() || `Custom rule for ${name}`,
      enabled: true,
      priority: 99,
      matchType,
      matchValue,
      transformType,
      transformValue,
    });
  };

  return (
    <div className="p-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
          New Rule
        </span>
        <button onClick={onCancel} className="p-1 text-gray-400 hover:text-gray-600">
          <X size={14} />
        </button>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="block text-xs text-gray-500 mb-1">Rule Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="e.g., Screenshots"
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          />
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Description</label>
          <input
            type="text"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What this rule does"
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          />
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Match Type</label>
          <select
            value={matchType}
            onChange={(e) => setMatchType(e.target.value as CustomRenameRule["matchType"])}
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          >
            {MATCH_TYPE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Match Value</label>
          <input
            type="text"
            value={matchValue}
            onChange={(e) => setMatchValue(e.target.value)}
            placeholder={MATCH_TYPE_OPTIONS.find((m) => m.value === matchType)?.example}
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          />
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Transform</label>
          <select
            value={transformType}
            onChange={(e) =>
              setTransformType(e.target.value as CustomRenameRule["transformType"])
            }
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          >
            {TRANSFORM_TYPE_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-gray-500 mb-1">Transform Value</label>
          <input
            type="text"
            value={transformValue}
            onChange={(e) => setTransformValue(e.target.value)}
            placeholder={
              transformType === "ai-prompt"
                ? "Custom AI instructions..."
                : TRANSFORM_TYPE_OPTIONS.find((t) => t.value === transformType)?.example
            }
            className="w-full px-2 py-1.5 text-sm rounded border border-gray-200 dark:border-gray-600 bg-white dark:bg-gray-700"
          />
        </div>
      </div>

      <div className="flex justify-end gap-2">
        <button
          onClick={onCancel}
          className="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800"
        >
          Cancel
        </button>
        <button
          onClick={handleSubmit}
          disabled={!name.trim()}
          className="px-3 py-1.5 text-sm bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Add Rule
        </button>
      </div>
    </div>
  );
}
