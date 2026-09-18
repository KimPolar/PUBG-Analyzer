export interface PubgMapDefinition {
  displayName: string;
  imageFile: string;
  worldSizeM: number;
}

const MAPS = {
  erangel: { displayName: "Erangel", imageFile: "erangel.webp", worldSizeM: 8_160 },
  miramar: { displayName: "Miramar", imageFile: "miramar.webp", worldSizeM: 8_160 },
  sanhok: { displayName: "Sanhok", imageFile: "sanhok.webp", worldSizeM: 4_080 },
  vikendi: { displayName: "Vikendi", imageFile: "vikendi.webp", worldSizeM: 8_160 },
  karakin: { displayName: "Karakin", imageFile: "karakin.webp", worldSizeM: 2_040 },
  paramo: { displayName: "Paramo", imageFile: "paramo.webp", worldSizeM: 3_060 },
  taego: { displayName: "Taego", imageFile: "taego.webp", worldSizeM: 8_160 },
  deston: { displayName: "Deston", imageFile: "deston.webp", worldSizeM: 8_160 },
  haven: { displayName: "Haven", imageFile: "haven.webp", worldSizeM: 1_020 },
  rondo: { displayName: "Rondo", imageFile: "rondo.webp", worldSizeM: 8_160 },
  campJackal: { displayName: "Camp Jackal", imageFile: "camp-jackal.webp", worldSizeM: 2_040 },
} satisfies Record<string, PubgMapDefinition>;

const MAP_BY_TELEMETRY_NAME: Readonly<Record<string, PubgMapDefinition>> = {
  Baltic_Main: MAPS.erangel,
  Erangel_Main: MAPS.erangel,
  Desert_Main: MAPS.miramar,
  Savage_Main: MAPS.sanhok,
  DihorOtok_Main: MAPS.vikendi,
  Summerland_Main: MAPS.karakin,
  Chimera_Main: MAPS.paramo,
  Tiger_Main: MAPS.taego,
  Kiki_Main: MAPS.deston,
  Heaven_Main: MAPS.haven,
  Neon_Main: MAPS.rondo,
  Range_Main: MAPS.campJackal,
  Training_Main: MAPS.campJackal,
};

export function getPubgMapDefinition(mapName: string): PubgMapDefinition | undefined {
  return MAP_BY_TELEMETRY_NAME[mapName];
}

export function getPubgMapImageUrl(map: PubgMapDefinition): string {
  return `${import.meta.env.BASE_URL}maps/${map.imageFile}`;
}

export function formatPubgMapName(mapName: string): string {
  return getPubgMapDefinition(mapName)?.displayName ?? mapName.replace(/_Main$/u, "");
}
