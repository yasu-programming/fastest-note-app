export interface Folder {
  id: string;
  name: string;
  parent_id: string | null;
  path: string;
  user_id: string;
  created_at: string;
  updated_at: string;
  note_count?: number;
  subfolder_count?: number;
}

export interface CreateFolderRequest {
  name: string;
  parent_id?: string | null;
}

export interface UpdateFolderRequest {
  name?: string;
  parent_id?: string | null;
}

export interface MoveFolderRequest {
  parent_id: string | null;
}

export interface FolderListResponse {
  folders: Folder[];
  total: number;
}

export interface FolderHierarchy extends Folder {
  children: FolderHierarchy[];
  depth: number;
}