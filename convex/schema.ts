import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  // User profile - linked to Clerk auth provider
  users: defineTable({
    name: v.string(),
    email: v.string(),
    tokenIdentifier: v.string(), // From Clerk (e.g., "clerk|user_xxx")
    avatarUrl: v.optional(v.string()),
    createdAt: v.number(),
  })
    .index("by_token", ["tokenIdentifier"])
    .index("by_email", ["email"]),

  // User settings - synced across devices
  userSettings: defineTable({
    userId: v.id("users"),
    // Appearance
    theme: v.union(v.literal("light"), v.literal("dark"), v.literal("system")),
    // Auto-rename sentinel
    autoRenameEnabled: v.boolean(),
    watchDownloads: v.optional(v.boolean()), // Whether to watch Downloads folder for auto-rename (optional for migration)
    watchedFolders: v.array(v.string()), // Additional paths to watch for auto-rename
    // File browser preferences
    showHiddenFiles: v.boolean(),
    defaultView: v.union(
      v.literal("list"),
      v.literal("grid"),
      v.literal("columns")
    ),
    sortBy: v.union(
      v.literal("name"),
      v.literal("date"),
      v.literal("size"),
      v.literal("type")
    ),
    sortDirection: v.union(v.literal("asc"), v.literal("desc")),
    // AI preferences
    aiModel: v.union(v.literal("haiku"), v.literal("sonnet")),
  }).index("by_user", ["userId"]),

  // Organization history - track AI organize operations
  organizeHistory: defineTable({
    userId: v.id("users"),
    folderPath: v.string(),
    folderName: v.string(),
    operationCount: v.number(),
    operations: v.array(
      v.object({
        type: v.union(
          v.literal("create_folder"),
          v.literal("move"),
          v.literal("rename"),
          v.literal("trash")
        ),
        sourcePath: v.string(),
        destPath: v.optional(v.string()),
      })
    ),
    completedAt: v.number(),
    summary: v.string(),
    wasUndone: v.boolean(),
  })
    .index("by_user", ["userId"])
    .index("by_user_date", ["userId", "completedAt"]),

  // Rename history - track auto-rename operations
  renameHistory: defineTable({
    userId: v.id("users"),
    originalName: v.string(),
    newName: v.string(),
    filePath: v.string(),
    fileSize: v.optional(v.number()),
    mimeType: v.optional(v.string()),
    renamedAt: v.number(),
    wasUndone: v.boolean(),
    aiModel: v.string(), // Which model suggested the rename
  })
    .index("by_user", ["userId"])
    .index("by_user_date", ["userId", "renamedAt"]),

  // Usage analytics - track API usage for billing awareness
  usageStats: defineTable({
    userId: v.id("users"),
    month: v.string(), // "2025-01" format
    organizeCount: v.number(),
    renameCount: v.number(),
    tokensUsed: v.number(),
  })
    .index("by_user", ["userId"])
    .index("by_user_month", ["userId", "month"]),
});
