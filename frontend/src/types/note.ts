export interface Note {
  id: string;
  title: string;
  content: string;
  folder_id: string | null;
  user_id: string;
  version: number;
  created_at: string;
  updated_at: string;
  size_bytes?: number;
}

export interface CreateNoteRequest {
  title: string;
  content: string;
  folder_id?: string | null;
}

export interface UpdateNoteRequest {
  title?: string;
  content?: string;
  folder_id?: string | null;
  version: number;
}

export interface MoveNoteRequest {
  folder_id: string | null;
}

export interface NoteListRequest {
  folder_id?: string;
  search?: string;
  limit?: number;
  offset?: number;
  sort_by?: 'created_at' | 'updated_at' | 'title';
  sort_order?: 'asc' | 'desc';
}

export interface NoteListResponse {
  notes: Note[];
  total: number;
  has_more: boolean;
}

export interface NoteSearchResult {
  note: Note;
  matches: {
    field: 'title' | 'content';
    snippet: string;
    highlights: Array<{ start: number; end: number }>;
  }[];
}