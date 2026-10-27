// FIVB VIS (issue #60). Mirrors crates/vb-vis.

export interface VisTournament {
  no: number;
  code: string;
  name: string;
  season: string;
  startDate: string | null;
  endDate: string | null;
}

export interface VisTeam {
  no: number;
  code: string;
  name: string;
}

export interface VisMatch {
  no: number;
  noInTournament: number | null;
  /** Local `YYYY-MM-DDTHH:MM:SS`. */
  dateTimeLocal: string;
  city: string;
  hall: string;
  /** Team A (home) first. */
  teams: [VisTeam, VisTeam];
}

/** Tournaments matching `query` (name or code, any order of words), newest first. */
export function findTournaments(all: VisTournament[], query: string): VisTournament[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return all
    .filter((t) => {
      const text = `${t.name} ${t.code} ${t.season}`.toLowerCase();
      return words.every((w) => text.includes(w));
    })
    .sort((a, b) => (b.startDate ?? "").localeCompare(a.startDate ?? "") || b.no - a.no);
}

/** `2027-03-14 19:45` from VIS's local date and time. */
export function when(m: VisMatch): string {
  return m.dateTimeLocal.replace("T", " ").slice(0, 16);
}
