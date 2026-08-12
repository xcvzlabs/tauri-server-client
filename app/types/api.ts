import * as v from 'valibot';

export type ApiErrorBody = {
  error: {
    code: string;
    message: string;
  };
};

export type UserRole = 'admin' | 'member';

export type MeProfile = {
  id: string;
  username: string;
  email: string;
  display_name: string;
  role: UserRole;
  is_online: boolean;
  last_seen_at: string | null;
  created_at: string;
};

export type UserProfile = {
  id: string;
  username: string;
  display_name: string;
  role: UserRole;
  is_online: boolean;
  last_seen_at: string | null;
};

export type TokenResponse = {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: MeProfile;
};

const presenceSnapshotEventSchema = v.object({
  type: v.literal('snapshot'),
  online_user_ids: v.array(v.string()),
});

const presenceUserEventSchema = v.object({
  type: v.union([v.literal('user_online'), v.literal('user_offline')]),
  user_id: v.string(),
  username: v.string(),
});

export const presenceEventSchema = v.union([presenceSnapshotEventSchema, presenceUserEventSchema]);

export type PresenceSnapshotEvent = v.InferOutput<typeof presenceSnapshotEventSchema>;
export type PresenceUserEvent = v.InferOutput<typeof presenceUserEventSchema>;
export type PresenceEvent = v.InferOutput<typeof presenceEventSchema>;
