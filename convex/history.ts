import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

// ============================================
// ORGANIZE HISTORY
// ============================================

/**
 * Record a completed organize operation
 */
export const recordOrganize = mutation({
  args: {
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
    summary: v.string(),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      throw new Error("User not found");
    }

    const historyId = await ctx.db.insert("organizeHistory", {
      userId: user._id,
      folderPath: args.folderPath,
      folderName: args.folderName,
      operationCount: args.operationCount,
      operations: args.operations,
      completedAt: Date.now(),
      summary: args.summary,
      wasUndone: false,
    });

    // Update usage stats
    await incrementUsageStat(ctx, user._id, "organizeCount");

    return historyId;
  },
});

/**
 * Get recent organize history
 */
export const getOrganizeHistory = query({
  args: {
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      return [];
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      return [];
    }

    const limit = args.limit ?? 50;

    return await ctx.db
      .query("organizeHistory")
      .withIndex("by_user_date", (q) => q.eq("userId", user._id))
      .order("desc")
      .take(limit);
  },
});

/**
 * Mark an organize operation as undone
 */
export const markOrganizeUndone = mutation({
  args: { historyId: v.id("organizeHistory") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const history = await ctx.db.get(args.historyId);
    if (!history) {
      throw new Error("History entry not found");
    }

    await ctx.db.patch(args.historyId, { wasUndone: true });
    return args.historyId;
  },
});

// ============================================
// RENAME HISTORY
// ============================================

/**
 * Record a rename operation
 */
export const recordRename = mutation({
  args: {
    originalName: v.string(),
    newName: v.string(),
    filePath: v.string(),
    fileSize: v.optional(v.number()),
    mimeType: v.optional(v.string()),
    aiModel: v.string(),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      throw new Error("User not found");
    }

    const historyId = await ctx.db.insert("renameHistory", {
      userId: user._id,
      originalName: args.originalName,
      newName: args.newName,
      filePath: args.filePath,
      fileSize: args.fileSize,
      mimeType: args.mimeType,
      renamedAt: Date.now(),
      wasUndone: false,
      aiModel: args.aiModel,
    });

    // Update usage stats
    await incrementUsageStat(ctx, user._id, "renameCount");

    return historyId;
  },
});

/**
 * Get recent rename history
 */
export const getRenameHistory = query({
  args: {
    limit: v.optional(v.number()),
  },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      return [];
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      return [];
    }

    const limit = args.limit ?? 50;

    return await ctx.db
      .query("renameHistory")
      .withIndex("by_user_date", (q) => q.eq("userId", user._id))
      .order("desc")
      .take(limit);
  },
});

/**
 * Mark a rename as undone
 */
export const markRenameUndone = mutation({
  args: { historyId: v.id("renameHistory") },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const history = await ctx.db.get(args.historyId);
    if (!history) {
      throw new Error("History entry not found");
    }

    await ctx.db.patch(args.historyId, { wasUndone: true });
    return args.historyId;
  },
});

// ============================================
// USAGE STATS
// ============================================

/**
 * Get usage stats for current month
 */
export const getCurrentUsageStats = query({
  args: {},
  handler: async (ctx) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      return null;
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      return null;
    }

    const currentMonth = getCurrentMonth();

    return await ctx.db
      .query("usageStats")
      .withIndex("by_user_month", (q) =>
        q.eq("userId", user._id).eq("month", currentMonth)
      )
      .unique();
  },
});

/**
 * Get usage stats for all time
 */
export const getAllTimeUsageStats = query({
  args: {},
  handler: async (ctx) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      return null;
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      return null;
    }

    const allStats = await ctx.db
      .query("usageStats")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .collect();

    return {
      organizeCount: allStats.reduce((sum, s) => sum + s.organizeCount, 0),
      renameCount: allStats.reduce((sum, s) => sum + s.renameCount, 0),
      tokensUsed: allStats.reduce((sum, s) => sum + s.tokensUsed, 0),
      monthlyBreakdown: allStats,
    };
  },
});

/**
 * Record token usage
 */
export const recordTokenUsage = mutation({
  args: { tokens: v.number() },
  handler: async (ctx, args) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) {
      throw new Error("Not authenticated");
    }

    const user = await ctx.db
      .query("users")
      .withIndex("by_token", (q) => q.eq("tokenIdentifier", identity.tokenIdentifier))
      .unique();

    if (!user) {
      throw new Error("User not found");
    }

    const currentMonth = getCurrentMonth();
    const stats = await ctx.db
      .query("usageStats")
      .withIndex("by_user_month", (q) =>
        q.eq("userId", user._id).eq("month", currentMonth)
      )
      .unique();

    if (stats) {
      await ctx.db.patch(stats._id, {
        tokensUsed: stats.tokensUsed + args.tokens,
      });
    } else {
      await ctx.db.insert("usageStats", {
        userId: user._id,
        month: currentMonth,
        organizeCount: 0,
        renameCount: 0,
        tokensUsed: args.tokens,
      });
    }
  },
});

// ============================================
// HELPERS
// ============================================

function getCurrentMonth(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
}

async function incrementUsageStat(
  ctx: { db: { query: Function; patch: Function; insert: Function } },
  userId: string,
  field: "organizeCount" | "renameCount"
) {
  const currentMonth = getCurrentMonth();

  const stats = await ctx.db
    .query("usageStats")
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    .withIndex("by_user_month", (q: any) =>
      q.eq("userId", userId).eq("month", currentMonth)
    )
    .unique();

  if (stats) {
    await ctx.db.patch(stats._id, {
      [field]: stats[field] + 1,
    });
  } else {
    await ctx.db.insert("usageStats", {
      userId,
      month: currentMonth,
      organizeCount: field === "organizeCount" ? 1 : 0,
      renameCount: field === "renameCount" ? 1 : 0,
      tokensUsed: 0,
    });
  }
}
