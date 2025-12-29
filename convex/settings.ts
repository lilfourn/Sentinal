import { v } from "convex/values";
import { mutation, query } from "./_generated/server";

/**
 * Get current user's settings
 */
export const getSettings = query({
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

    return await ctx.db
      .query("userSettings")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .unique();
  },
});

/**
 * Update user settings (partial update)
 */
export const updateSettings = mutation({
  args: {
    theme: v.optional(v.union(v.literal("light"), v.literal("dark"), v.literal("system"))),
    autoRenameEnabled: v.optional(v.boolean()),
    watchedFolders: v.optional(v.array(v.string())),
    showHiddenFiles: v.optional(v.boolean()),
    defaultView: v.optional(
      v.union(v.literal("list"), v.literal("grid"), v.literal("columns"))
    ),
    sortBy: v.optional(
      v.union(v.literal("name"), v.literal("date"), v.literal("size"), v.literal("type"))
    ),
    sortDirection: v.optional(v.union(v.literal("asc"), v.literal("desc"))),
    aiModel: v.optional(v.union(v.literal("haiku"), v.literal("sonnet"))),
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

    const settings = await ctx.db
      .query("userSettings")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .unique();

    if (!settings) {
      throw new Error("Settings not found");
    }

    // Build update object with only provided fields
    const updates: Record<string, unknown> = {};
    if (args.theme !== undefined) updates.theme = args.theme;
    if (args.autoRenameEnabled !== undefined) updates.autoRenameEnabled = args.autoRenameEnabled;
    if (args.watchedFolders !== undefined) updates.watchedFolders = args.watchedFolders;
    if (args.showHiddenFiles !== undefined) updates.showHiddenFiles = args.showHiddenFiles;
    if (args.defaultView !== undefined) updates.defaultView = args.defaultView;
    if (args.sortBy !== undefined) updates.sortBy = args.sortBy;
    if (args.sortDirection !== undefined) updates.sortDirection = args.sortDirection;
    if (args.aiModel !== undefined) updates.aiModel = args.aiModel;

    await ctx.db.patch(settings._id, updates);
    return settings._id;
  },
});

/**
 * Add a watched folder for auto-rename
 */
export const addWatchedFolder = mutation({
  args: { folderPath: v.string() },
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

    const settings = await ctx.db
      .query("userSettings")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .unique();

    if (!settings) {
      throw new Error("Settings not found");
    }

    // Avoid duplicates
    if (!settings.watchedFolders.includes(args.folderPath)) {
      await ctx.db.patch(settings._id, {
        watchedFolders: [...settings.watchedFolders, args.folderPath],
      });
    }

    return settings._id;
  },
});

/**
 * Remove a watched folder
 */
export const removeWatchedFolder = mutation({
  args: { folderPath: v.string() },
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

    const settings = await ctx.db
      .query("userSettings")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .unique();

    if (!settings) {
      throw new Error("Settings not found");
    }

    await ctx.db.patch(settings._id, {
      watchedFolders: settings.watchedFolders.filter((f) => f !== args.folderPath),
    });

    return settings._id;
  },
});

/**
 * Toggle auto-rename feature
 */
export const toggleAutoRename = mutation({
  args: { enabled: v.boolean() },
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

    const settings = await ctx.db
      .query("userSettings")
      .withIndex("by_user", (q) => q.eq("userId", user._id))
      .unique();

    if (!settings) {
      throw new Error("Settings not found");
    }

    await ctx.db.patch(settings._id, {
      autoRenameEnabled: args.enabled,
    });

    return settings._id;
  },
});
