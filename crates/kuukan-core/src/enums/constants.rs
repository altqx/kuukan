//! `Jikan\Helper\Constants` frozen as `pub const`s.
//!
//! Verbatim port of `/tmp/opencode/jikan-php/src/Helper/Constants.php`
//! (jikan-php v4.0.12). The values are mal_id ids: anime/manga genres, themes
//! and demographics share the `GENRE_*` ranges (e.g. `SCHOOL`/`MUSIC` are
//! themes, `SEINEN`/`JOSEI`/`KIDS` are demographics), MAL search/order codes,
//! user-list status/order codes and review sort labels.
//!
//! Notes preserved from the PHP source: several constants are MAL renames and
//! kept as aliases (`GENRE_ANIME_CARS` == `RACING`), and some ids were removed
//! or merged by MAL (e.g. `GENRE_ANIME_MAGIC`) but kept for reference.

// ---------------------------------------------------------------------------
// Base URLs
// ---------------------------------------------------------------------------

/// `Constants::BASE_URL`.
pub const BASE_URL: &str = "https://myanimelist.net";
/// `Constants::CDN_URL`.
pub const CDN_URL: &str = "https://cdn.myanimelist.net";

// ---------------------------------------------------------------------------
// Seasons
// ---------------------------------------------------------------------------

/// `Constants::SEASONS`.
pub const SEASONS: [&str; 4] = ["Winter", "Spring", "Summer", "Fall"];

/// `Constants::WINTER`.
pub const WINTER: &str = "winter";
/// `Constants::SPRING`.
pub const SPRING: &str = "spring";
/// `Constants::SUMMER`.
pub const SUMMER: &str = "summer";
/// `Constants::FALL`.
pub const FALL: &str = "fall";

// ---------------------------------------------------------------------------
// Entity kinds
// ---------------------------------------------------------------------------

/// `Constants::ANIME`.
pub const ANIME: &str = "anime";
/// `Constants::MANGA`.
pub const MANGA: &str = "manga";
/// `Constants::CHARACTER`.
pub const CHARACTER: &str = "character";
/// `Constants::PERSON`.
pub const PERSON: &str = "person";

// ---------------------------------------------------------------------------
// Top / recent
// ---------------------------------------------------------------------------

/// `Constants::TOP_AIRING`.
pub const TOP_AIRING: &str = "airing";
/// `Constants::TOP_UPCOMING`.
pub const TOP_UPCOMING: &str = "upcoming";
/// `Constants::TOP_TV`.
pub const TOP_TV: &str = "tv";
/// `Constants::TOP_MOVIE`.
pub const TOP_MOVIE: &str = "movie";
/// `Constants::TOP_OVA`.
pub const TOP_OVA: &str = "ova";
/// `Constants::TOP_SPECIAL`.
pub const TOP_SPECIAL: &str = "special";
/// `Constants::TOP_ONA`.
pub const TOP_ONA: &str = "ona";

/// `Constants::TOP_MANGA`.
pub const TOP_MANGA: &str = "manga";
/// `Constants::TOP_NOVEL`.
pub const TOP_NOVEL: &str = "novels";
/// `Constants::TOP_ONE_SHOT`.
pub const TOP_ONE_SHOT: &str = "oneshots";
/// `Constants::TOP_DOUJINSHI`.
pub const TOP_DOUJINSHI: &str = "doujin";
/// `Constants::TOP_MANHWA`.
pub const TOP_MANHWA: &str = "manhwa";
/// `Constants::TOP_MANHUA`.
pub const TOP_MANHUA: &str = "manhua";
/// `Constants::TOP_LIGHTNOVELS`.
pub const TOP_LIGHTNOVELS: &str = "lightnovels";

/// `Constants::TOP_BY_POPULARITY`.
pub const TOP_BY_POPULARITY: &str = "bypopularity";
/// `Constants::TOP_BY_FAVORITES`.
pub const TOP_BY_FAVORITES: &str = "favorite";

/// `Constants::RECENT_RECOMMENDATION_ANIME`.
pub const RECENT_RECOMMENDATION_ANIME: &str = "anime";
/// `Constants::RECENT_RECOMMENDATION_MANGA`.
pub const RECENT_RECOMMENDATION_MANGA: &str = "manga";

// ---------------------------------------------------------------------------
// v3 status constants (kept by the PHP source)
// ---------------------------------------------------------------------------

/// `Constants::STATUS_ANIME_AIRING`.
pub const STATUS_ANIME_AIRING: i32 = 1;
/// `Constants::STATUS_ANIME_FINISHED`.
pub const STATUS_ANIME_FINISHED: i32 = 2;
/// `Constants::STATUS_ANIME_NOT_YET_AIRED`.
pub const STATUS_ANIME_NOT_YET_AIRED: i32 = 3;
/// `Constants::STATUS_MANGA_PUBLISHING`.
pub const STATUS_MANGA_PUBLISHING: i32 = 1;
/// `Constants::STATUS_MANGA_FINISHED`.
pub const STATUS_MANGA_FINISHED: i32 = 2;
/// `Constants::STATUS_MANGA_NOT_YET_PUBLISHED`.
pub const STATUS_MANGA_NOT_YET_PUBLISHED: i32 = 3;
/// `Constants::STATUS_MANGA_ON_HIATUS`.
pub const STATUS_MANGA_ON_HIATUS: i32 = 4;
/// `Constants::STATUS_MANGA_DISCONTINUED`.
pub const STATUS_MANGA_DISCONTINUED: i32 = 5;

/// `Constants::TOP_REVIEW_ANIME`.
pub const TOP_REVIEW_ANIME: &str = "anime";
/// `Constants::TOP_REVIEW_MANGA`.
pub const TOP_REVIEW_MANGA: &str = "manga";
/// `Constants::TOP_REVIEW_BEST_VOTED`.
pub const TOP_REVIEW_BEST_VOTED: &str = "bestvoted";

// ---------------------------------------------------------------------------
// Legacy search codes
// ---------------------------------------------------------------------------

/// `Constants::SEARCH_ANIME_TV`.
pub const SEARCH_ANIME_TV: i32 = 1;
/// `Constants::SEARCH_ANIME_OVA`.
pub const SEARCH_ANIME_OVA: i32 = 2;
/// `Constants::SEARCH_ANIME_MOVIE`.
pub const SEARCH_ANIME_MOVIE: i32 = 3;
/// `Constants::SEARCH_ANIME_SPECIAL`.
pub const SEARCH_ANIME_SPECIAL: i32 = 4;
/// `Constants::SEARCH_ANIME_ONA`.
pub const SEARCH_ANIME_ONA: i32 = 5;
/// `Constants::SEARCH_ANIME_MUSIC`.
pub const SEARCH_ANIME_MUSIC: i32 = 6;

/// `Constants::SEARCH_MANGA_MANGA`.
pub const SEARCH_MANGA_MANGA: i32 = 1;
/// `Constants::SEARCH_MANGA_NOVEL`.
pub const SEARCH_MANGA_NOVEL: i32 = 2;
/// `Constants::SEARCH_MANGA_ONESHOT`.
pub const SEARCH_MANGA_ONESHOT: i32 = 3;
/// `Constants::SEARCH_MANGA_DOUJIN`.
pub const SEARCH_MANGA_DOUJIN: i32 = 4;
/// `Constants::SEARCH_MANGA_MANHWA`.
pub const SEARCH_MANGA_MANHWA: i32 = 5;
/// `Constants::SEARCH_MANGA_MANHUA`.
pub const SEARCH_MANGA_MANHUA: i32 = 6;

/// `Constants::SEARCH_ANIME_STATUS_AIRING`.
pub const SEARCH_ANIME_STATUS_AIRING: i32 = 1;
/// `Constants::SEARCH_ANIME_STATUS_FINISHED_AIRING`.
pub const SEARCH_ANIME_STATUS_FINISHED_AIRING: i32 = 2;
/// `Constants::SEARCH_ANIME_STATUS_COMPLETED` (alias of finished airing).
pub const SEARCH_ANIME_STATUS_COMPLETED: i32 = 2;
/// `Constants::SEARCH_ANIME_STATUS_TO_BE_AIRD`.
pub const SEARCH_ANIME_STATUS_TO_BE_AIRD: i32 = 3;
/// `Constants::SEARCH_ANIME_STATUS_TBA` (alias of to-be-aired).
pub const SEARCH_ANIME_STATUS_TBA: i32 = 3;

/// `Constants::SEARCH_MANGA_STATUS_PUBLISHING`.
pub const SEARCH_MANGA_STATUS_PUBLISHING: i32 = 1;
/// `Constants::SEARCH_MANGA_STATUS_FINISHED_PUBLISHING`.
pub const SEARCH_MANGA_STATUS_FINISHED_PUBLISHING: i32 = 2;
/// `Constants::SEARCH_MANGA_STATUS_COMPLETED` (alias of finished publishing).
pub const SEARCH_MANGA_STATUS_COMPLETED: i32 = 2;
/// `Constants::SEARCH_MANGA_STATUS_TO_BE_PUBLISHED`.
pub const SEARCH_MANGA_STATUS_TO_BE_PUBLISHED: i32 = 3;
/// `Constants::SEARCH_MANGA_STATUS_TBP` (alias of to-be-published).
pub const SEARCH_MANGA_STATUS_TBP: i32 = 3;

/// `Constants::SEARCH_ANIME_RATING_G`.
pub const SEARCH_ANIME_RATING_G: i32 = 1;
/// `Constants::SEARCH_ANIME_RATING_ALL` (alias of G).
pub const SEARCH_ANIME_RATING_ALL: i32 = 1;
/// `Constants::SEARCH_ANIME_RATING_PG`.
pub const SEARCH_ANIME_RATING_PG: i32 = 2;
/// `Constants::SEARCH_ANIME_RATING_PG13`.
pub const SEARCH_ANIME_RATING_PG13: i32 = 3;
/// `Constants::SEARCH_ANIME_RATING_R17`.
pub const SEARCH_ANIME_RATING_R17: i32 = 4;
/// `Constants::SEARCH_ANIME_RATING_R`.
pub const SEARCH_ANIME_RATING_R: i32 = 5;
/// `Constants::SEARCH_ANIME_RATING_RX`.
pub const SEARCH_ANIME_RATING_RX: i32 = 6;
/// `Constants::SEARCH_ANIME_RATING_HENTAI` (alias of Rx).
pub const SEARCH_ANIME_RATING_HENTAI: i32 = 6;

/// `Constants::SEARCH_SORT_ASCENDING`.
pub const SEARCH_SORT_ASCENDING: i32 = 0;
/// `Constants::SEARCH_SORT_DESCENDING`.
pub const SEARCH_SORT_DESCENDING: i32 = 1;

/// `Constants::SEARCH_ANIME_ORDER_BY_TITLE`.
pub const SEARCH_ANIME_ORDER_BY_TITLE: i32 = 0;
/// `Constants::SEARCH_ANIME_ORDER_BY_START_DATE`.
pub const SEARCH_ANIME_ORDER_BY_START_DATE: i32 = 2;
/// `Constants::SEARCH_ANIME_ORDER_BY_SCORE`.
pub const SEARCH_ANIME_ORDER_BY_SCORE: i32 = 3;
/// `Constants::SEARCH_ANIME_ORDER_BY_EPISODES`.
pub const SEARCH_ANIME_ORDER_BY_EPISODES: i32 = 4;
/// `Constants::SEARCH_ANIME_ORDER_BY_END_DATE`.
pub const SEARCH_ANIME_ORDER_BY_END_DATE: i32 = 5;
/// `Constants::SEARCH_ANIME_ORDER_BY_TYPE`.
pub const SEARCH_ANIME_ORDER_BY_TYPE: i32 = 6;
/// `Constants::SEARCH_ANIME_ORDER_BY_MEMBERS`.
pub const SEARCH_ANIME_ORDER_BY_MEMBERS: i32 = 7;
/// `Constants::SEARCH_ANIME_ORDER_BY_RATED`.
pub const SEARCH_ANIME_ORDER_BY_RATED: i32 = 8;
/// `Constants::SEARCH_ANIME_ORDER_BY_ID`.
pub const SEARCH_ANIME_ORDER_BY_ID: i32 = 9;

/// `Constants::SEARCH_MANGA_ORDER_BY_TITLE`.
pub const SEARCH_MANGA_ORDER_BY_TITLE: i32 = 0;
/// `Constants::SEARCH_MANGA_ORDER_BY_START_DATE`.
pub const SEARCH_MANGA_ORDER_BY_START_DATE: i32 = 2;
/// `Constants::SEARCH_MANGA_ORDER_BY_SCORE`.
pub const SEARCH_MANGA_ORDER_BY_SCORE: i32 = 3;
/// `Constants::SEARCH_MANGA_ORDER_BY_VOLUMES`.
pub const SEARCH_MANGA_ORDER_BY_VOLUMES: i32 = 4;
/// `Constants::SEARCH_MANGA_ORDER_BY_END_DATE`.
pub const SEARCH_MANGA_ORDER_BY_END_DATE: i32 = 5;
/// `Constants::SEARCH_MANGA_ORDER_BY_CHAPTERS`.
pub const SEARCH_MANGA_ORDER_BY_CHAPTERS: i32 = 6;
/// `Constants::SEARCH_MANGA_ORDER_BY_MEMBERS`.
pub const SEARCH_MANGA_ORDER_BY_MEMBERS: i32 = 7;
/// `Constants::SEARCH_MANGA_ORDER_BY_TYPE`.
pub const SEARCH_MANGA_ORDER_BY_TYPE: i32 = 8;
/// `Constants::SEARCH_MANGA_ORDER_BY_ID`.
pub const SEARCH_MANGA_ORDER_BY_ID: i32 = 9;

/// `Constants::SEARCH_USER_GENDER_ANY`.
pub const SEARCH_USER_GENDER_ANY: i32 = -1;
/// `Constants::SEARCH_USER_GENDER_MALE`.
pub const SEARCH_USER_GENDER_MALE: i32 = 1;
/// `Constants::SEARCH_USER_GENDER_FEMALE`.
pub const SEARCH_USER_GENDER_FEMALE: i32 = 2;
/// `Constants::SEARCH_USER_GENDER_NONBINARY`.
pub const SEARCH_USER_GENDER_NONBINARY: i32 = 3;

// ---------------------------------------------------------------------------
// Anime genres / themes / demographics
// ---------------------------------------------------------------------------

/// `Constants::GENRE_ANIME_ACTION`.
pub const GENRE_ANIME_ACTION: i32 = 1;
/// `Constants::GENRE_ANIME_ADVENTURE`.
pub const GENRE_ANIME_ADVENTURE: i32 = 2;
/// `Constants::GENRE_ANIME_RACING`.
pub const GENRE_ANIME_RACING: i32 = 3;
/// `Constants::GENRE_ANIME_CARS` (renamed to racing by MAL).
pub const GENRE_ANIME_CARS: i32 = 3;
/// `Constants::GENRE_ANIME_COMEDY`.
pub const GENRE_ANIME_COMEDY: i32 = 4;
/// `Constants::GENRE_ANIME_AVANT_GARDE`.
pub const GENRE_ANIME_AVANT_GARDE: i32 = 5;
/// `Constants::GENRE_ANIME_DEMENTIA` (renamed to avant garde by MAL).
pub const GENRE_ANIME_DEMENTIA: i32 = 5;
/// `Constants::GENRE_ANIME_MYTHOLOGY`.
pub const GENRE_ANIME_MYTHOLOGY: i32 = 6;
/// `Constants::GENRE_ANIME_DEMONS` (renamed to mythology by MAL).
pub const GENRE_ANIME_DEMONS: i32 = 6;
/// `Constants::GENRE_ANIME_MYSTERY`.
pub const GENRE_ANIME_MYSTERY: i32 = 7;
/// `Constants::GENRE_ANIME_DRAMA`.
pub const GENRE_ANIME_DRAMA: i32 = 8;
/// `Constants::GENRE_ANIME_ECCHI`.
pub const GENRE_ANIME_ECCHI: i32 = 9;
/// `Constants::GENRE_ANIME_FANTASY`.
pub const GENRE_ANIME_FANTASY: i32 = 10;
/// `Constants::GENRE_ANIME_STRATEGY_GAME`.
pub const GENRE_ANIME_STRATEGY_GAME: i32 = 11;
/// `Constants::GENRE_ANIME_GAME` (renamed to strategy game by MAL).
pub const GENRE_ANIME_GAME: i32 = 11;
/// `Constants::GENRE_ANIME_HENTAI`.
pub const GENRE_ANIME_HENTAI: i32 = 12;
/// `Constants::GENRE_ANIME_HISTORICAL`.
pub const GENRE_ANIME_HISTORICAL: i32 = 13;
/// `Constants::GENRE_ANIME_HORROR`.
pub const GENRE_ANIME_HORROR: i32 = 14;
/// `Constants::GENRE_ANIME_KIDS`.
pub const GENRE_ANIME_KIDS: i32 = 15;
/// `Constants::GENRE_ANIME_MAGIC` (removed by MAL - 404).
pub const GENRE_ANIME_MAGIC: i32 = 16;
/// `Constants::GENRE_ANIME_MARTIAL_ARTS`.
pub const GENRE_ANIME_MARTIAL_ARTS: i32 = 17;
/// `Constants::GENRE_ANIME_MECHA`.
pub const GENRE_ANIME_MECHA: i32 = 18;
/// `Constants::GENRE_ANIME_MUSIC`.
pub const GENRE_ANIME_MUSIC: i32 = 19;
/// `Constants::GENRE_ANIME_PARODY`.
pub const GENRE_ANIME_PARODY: i32 = 20;
/// `Constants::GENRE_ANIME_SAMURAI`.
pub const GENRE_ANIME_SAMURAI: i32 = 21;
/// `Constants::GENRE_ANIME_ROMANCE`.
pub const GENRE_ANIME_ROMANCE: i32 = 22;
/// `Constants::GENRE_ANIME_SCHOOL`.
pub const GENRE_ANIME_SCHOOL: i32 = 23;
/// `Constants::GENRE_ANIME_SCI_FI`.
pub const GENRE_ANIME_SCI_FI: i32 = 24;
/// `Constants::GENRE_ANIME_SHOUJO`.
pub const GENRE_ANIME_SHOUJO: i32 = 25;
/// `Constants::GENRE_ANIME_GIRLS_LOVE`.
pub const GENRE_ANIME_GIRLS_LOVE: i32 = 26;
/// `Constants::GENRE_ANIME_SHOUJO_AI` (renamed to girls love by MAL).
pub const GENRE_ANIME_SHOUJO_AI: i32 = 26;
/// `Constants::GENRE_ANIME_SHOUNEN`.
pub const GENRE_ANIME_SHOUNEN: i32 = 27;
/// `Constants::GENRE_ANIME_BOYS_LOVE`.
pub const GENRE_ANIME_BOYS_LOVE: i32 = 28;
/// `Constants::GENRE_ANIME_SHOUNEN_AI` (renamed to boys love by MAL).
pub const GENRE_ANIME_SHOUNEN_AI: i32 = 28;
/// `Constants::GENRE_ANIME_SPACE`.
pub const GENRE_ANIME_SPACE: i32 = 29;
/// `Constants::GENRE_ANIME_SPORTS`.
pub const GENRE_ANIME_SPORTS: i32 = 30;
/// `Constants::GENRE_ANIME_SUPER_POWER`.
pub const GENRE_ANIME_SUPER_POWER: i32 = 31;
/// `Constants::GENRE_ANIME_VAMPIRE`.
pub const GENRE_ANIME_VAMPIRE: i32 = 32;
/// `Constants::GENRE_ANIME_YAOI` (merged into boys love by MAL - 404).
pub const GENRE_ANIME_YAOI: i32 = 33;
/// `Constants::GENRE_ANIME_YURI` (merged into girls love by MAL - 404).
pub const GENRE_ANIME_YURI: i32 = 34;
/// `Constants::GENRE_ANIME_HAREM`.
pub const GENRE_ANIME_HAREM: i32 = 35;
/// `Constants::GENRE_ANIME_SLICE_OF_LIFE`.
pub const GENRE_ANIME_SLICE_OF_LIFE: i32 = 36;
/// `Constants::GENRE_ANIME_SUPERNATURAL`.
pub const GENRE_ANIME_SUPERNATURAL: i32 = 37;
/// `Constants::GENRE_ANIME_MILITARY`.
pub const GENRE_ANIME_MILITARY: i32 = 38;
/// `Constants::GENRE_ANIME_DETECTIVE`.
pub const GENRE_ANIME_DETECTIVE: i32 = 39;
/// `Constants::GENRE_ANIME_POLICE` (renamed to detective by MAL).
pub const GENRE_ANIME_POLICE: i32 = 39;
/// `Constants::GENRE_ANIME_PSYCHOLOGICAL`.
pub const GENRE_ANIME_PSYCHOLOGICAL: i32 = 40;
/// `Constants::GENRE_ANIME_SUSPENSE`.
pub const GENRE_ANIME_SUSPENSE: i32 = 41;
/// `Constants::GENRE_ANIME_THRILLER` (renamed to suspense by MAL).
pub const GENRE_ANIME_THRILLER: i32 = 41;
/// `Constants::GENRE_ANIME_SEINEN`.
pub const GENRE_ANIME_SEINEN: i32 = 42;
/// `Constants::GENRE_ANIME_JOSEI`.
pub const GENRE_ANIME_JOSEI: i32 = 43;
/// `Constants::GENRE_ANIME_AWARD_WINNING`.
pub const GENRE_ANIME_AWARD_WINNING: i32 = 46;
/// `Constants::GENRE_ANIME_GOURMET`.
pub const GENRE_ANIME_GOURMET: i32 = 47;
/// `Constants::GENRE_ANIME_WORKPLACE`.
pub const GENRE_ANIME_WORKPLACE: i32 = 48;
/// `Constants::GENRE_ANIME_WORK_LIFE` (renamed to workplace by MAL).
pub const GENRE_ANIME_WORK_LIFE: i32 = 48;
/// `Constants::GENRE_ANIME_EROTICA`.
pub const GENRE_ANIME_EROTICA: i32 = 49;
/// `Constants::GENRE_ANIME_ADULT_CAST`.
pub const GENRE_ANIME_ADULT_CAST: i32 = 50;
/// `Constants::GENRE_ANIME_ANTHROPOMORPHIC`.
pub const GENRE_ANIME_ANTHROPOMORPHIC: i32 = 51;
/// `Constants::GENRE_ANIME_CGDCT`.
pub const GENRE_ANIME_CGDCT: i32 = 52;
/// `Constants::GENRE_ANIME_CHILDCARE`.
pub const GENRE_ANIME_CHILDCARE: i32 = 53;
/// `Constants::GENRE_ANIME_COMBAT_SPORTS`.
pub const GENRE_ANIME_COMBAT_SPORTS: i32 = 54;
/// `Constants::GENRE_ANIME_DELINQUENTS`.
pub const GENRE_ANIME_DELINQUENTS: i32 = 55;
/// `Constants::GENRE_ANIME_EDUCATIONAL`.
pub const GENRE_ANIME_EDUCATIONAL: i32 = 56;
/// `Constants::GENRE_ANIME_GAG_HUMOR`.
pub const GENRE_ANIME_GAG_HUMOR: i32 = 57;
/// `Constants::GENRE_ANIME_GORE`.
pub const GENRE_ANIME_GORE: i32 = 58;
/// `Constants::GENRE_ANIME_HIGH_STAKES_GAME`.
pub const GENRE_ANIME_HIGH_STAKES_GAME: i32 = 59;
/// `Constants::GENRE_ANIME_IDOLS_FEMALE`.
pub const GENRE_ANIME_IDOLS_FEMALE: i32 = 60;
/// `Constants::GENRE_ANIME_IDOLS_MALE`.
pub const GENRE_ANIME_IDOLS_MALE: i32 = 61;
/// `Constants::GENRE_ANIME_ISEKAI`.
pub const GENRE_ANIME_ISEKAI: i32 = 62;
/// `Constants::GENRE_ANIME_IYASHIKEI`.
pub const GENRE_ANIME_IYASHIKEI: i32 = 63;
/// `Constants::GENRE_ANIME_LOVE_POLYGON`.
pub const GENRE_ANIME_LOVE_POLYGON: i32 = 64;
/// `Constants::GENRE_ANIME_MAGICAL_SEX_SHIFT`.
pub const GENRE_ANIME_MAGICAL_SEX_SHIFT: i32 = 65;
/// `Constants::GENRE_ANIME_MAHOU_SHOUJO`.
pub const GENRE_ANIME_MAHOU_SHOUJO: i32 = 66;
/// `Constants::GENRE_ANIME_MEDICAL`.
pub const GENRE_ANIME_MEDICAL: i32 = 67;
/// `Constants::GENRE_ANIME_ORGANIZED_CRIME`.
pub const GENRE_ANIME_ORGANIZED_CRIME: i32 = 68;
/// `Constants::GENRE_ANIME_OTAKU_CULTURE`.
pub const GENRE_ANIME_OTAKU_CULTURE: i32 = 69;
/// `Constants::GENRE_ANIME_PERFORMING_ARTS`.
pub const GENRE_ANIME_PERFORMING_ARTS: i32 = 70;
/// `Constants::GENRE_ANIME_PETS`.
pub const GENRE_ANIME_PETS: i32 = 71;
/// `Constants::GENRE_ANIME_REINCARNATION`.
pub const GENRE_ANIME_REINCARNATION: i32 = 72;
/// `Constants::GENRE_ANIME_REVERSE_HAREM`.
pub const GENRE_ANIME_REVERSE_HAREM: i32 = 73;
/// `Constants::GENRE_ANIME_ROMANTIC_SUBTEXT`.
pub const GENRE_ANIME_ROMANTIC_SUBTEXT: i32 = 74;
/// `Constants::GENRE_ANIME_SHOWBIZ`.
pub const GENRE_ANIME_SHOWBIZ: i32 = 75;
/// `Constants::GENRE_ANIME_SURVIVAL`.
pub const GENRE_ANIME_SURVIVAL: i32 = 76;
/// `Constants::GENRE_ANIME_TEAM_SPORTS`.
pub const GENRE_ANIME_TEAM_SPORTS: i32 = 77;
/// `Constants::GENRE_ANIME_TIME_TRAVEL`.
pub const GENRE_ANIME_TIME_TRAVEL: i32 = 78;
/// `Constants::GENRE_ANIME_VIDEO_GAME`.
pub const GENRE_ANIME_VIDEO_GAME: i32 = 79;
/// `Constants::GENRE_ANIME_VISUAL_ARTS`.
pub const GENRE_ANIME_VISUAL_ARTS: i32 = 80;
/// `Constants::GENRE_ANIME_CROSSDRESSING`.
pub const GENRE_ANIME_CROSSDRESSING: i32 = 81;

// ---------------------------------------------------------------------------
// Manga genres / themes / demographics
// ---------------------------------------------------------------------------

/// `Constants::GENRE_MANGA_ACTION`.
pub const GENRE_MANGA_ACTION: i32 = 1;
/// `Constants::GENRE_MANGA_ADVENTURE`.
pub const GENRE_MANGA_ADVENTURE: i32 = 2;
/// `Constants::GENRE_MANGA_RACING`.
pub const GENRE_MANGA_RACING: i32 = 3;
/// `Constants::GENRE_MANGA_CARS` (renamed to racing by MAL).
pub const GENRE_MANGA_CARS: i32 = 3;
/// `Constants::GENRE_MANGA_COMEDY`.
pub const GENRE_MANGA_COMEDY: i32 = 4;
/// `Constants::GENRE_MANGA_AVANT_GARDE`.
pub const GENRE_MANGA_AVANT_GARDE: i32 = 5;
/// `Constants::GENRE_MANGA_DEMENTIA` (renamed to avant garde by MAL).
pub const GENRE_MANGA_DEMENTIA: i32 = 5;
/// `Constants::GENRE_MANGA_MYTHOLOGY`.
pub const GENRE_MANGA_MYTHOLOGY: i32 = 6;
/// `Constants::GENRE_MANGA_DEMONS` (renamed to mythology by MAL).
pub const GENRE_MANGA_DEMONS: i32 = 6;
/// `Constants::GENRE_MANGA_MYSTERY`.
pub const GENRE_MANGA_MYSTERY: i32 = 7;
/// `Constants::GENRE_MANGA_DRAMA`.
pub const GENRE_MANGA_DRAMA: i32 = 8;
/// `Constants::GENRE_MANGA_ECCHI`.
pub const GENRE_MANGA_ECCHI: i32 = 9;
/// `Constants::GENRE_MANGA_FANTASY`.
pub const GENRE_MANGA_FANTASY: i32 = 10;
/// `Constants::GENRE_MANGA_STRATEGY_GAME`.
pub const GENRE_MANGA_STRATEGY_GAME: i32 = 11;
/// `Constants::GENRE_MANGA_GAME` (renamed to strategy game by MAL).
pub const GENRE_MANGA_GAME: i32 = 11;
/// `Constants::GENRE_MANGA_HENTAI`.
pub const GENRE_MANGA_HENTAI: i32 = 12;
/// `Constants::GENRE_MANGA_HISTORICAL`.
pub const GENRE_MANGA_HISTORICAL: i32 = 13;
/// `Constants::GENRE_MANGA_HORROR`.
pub const GENRE_MANGA_HORROR: i32 = 14;
/// `Constants::GENRE_MANGA_KIDS`.
pub const GENRE_MANGA_KIDS: i32 = 15;
/// `Constants::GENRE_MANGA_MAGIC` (removed by MAL - 404).
pub const GENRE_MANGA_MAGIC: i32 = 16;
/// `Constants::GENRE_MANGA_MARTIAL_ARTS`.
pub const GENRE_MANGA_MARTIAL_ARTS: i32 = 17;
/// `Constants::GENRE_MANGA_MECHA`.
pub const GENRE_MANGA_MECHA: i32 = 18;
/// `Constants::GENRE_MANGA_MUSIC`.
pub const GENRE_MANGA_MUSIC: i32 = 19;
/// `Constants::GENRE_MANGA_PARODY`.
pub const GENRE_MANGA_PARODY: i32 = 20;
/// `Constants::GENRE_MANGA_SAMURAI`.
pub const GENRE_MANGA_SAMURAI: i32 = 21;
/// `Constants::GENRE_MANGA_ROMANCE`.
pub const GENRE_MANGA_ROMANCE: i32 = 22;
/// `Constants::GENRE_MANGA_SCHOOL`.
pub const GENRE_MANGA_SCHOOL: i32 = 23;
/// `Constants::GENRE_MANGA_SCI_FI`.
pub const GENRE_MANGA_SCI_FI: i32 = 24;
/// `Constants::GENRE_MANGA_SHOUJO`.
pub const GENRE_MANGA_SHOUJO: i32 = 25;
/// `Constants::GENRE_MANGA_GIRLS_LOVE`.
pub const GENRE_MANGA_GIRLS_LOVE: i32 = 26;
/// `Constants::GENRE_MANGA_SHOUJO_AI` (renamed to girls love by MAL).
pub const GENRE_MANGA_SHOUJO_AI: i32 = 26;
/// `Constants::GENRE_MANGA_SHOUNEN`.
pub const GENRE_MANGA_SHOUNEN: i32 = 27;
/// `Constants::GENRE_MANGA_BOYS_LOVE`.
pub const GENRE_MANGA_BOYS_LOVE: i32 = 28;
/// `Constants::GENRE_MANGA_SHOUNEN_AI` (renamed to boys love by MAL).
pub const GENRE_MANGA_SHOUNEN_AI: i32 = 28;
/// `Constants::GENRE_MANGA_SPACE`.
pub const GENRE_MANGA_SPACE: i32 = 29;
/// `Constants::GENRE_MANGA_SPORTS`.
pub const GENRE_MANGA_SPORTS: i32 = 30;
/// `Constants::GENRE_MANGA_SUPER_POWER`.
pub const GENRE_MANGA_SUPER_POWER: i32 = 31;
/// `Constants::GENRE_MANGA_VAMPIRE`.
pub const GENRE_MANGA_VAMPIRE: i32 = 32;
/// `Constants::GENRE_MANGA_YAOI` (merged into boys love by MAL - 404).
pub const GENRE_MANGA_YAOI: i32 = 33;
/// `Constants::GENRE_MANGA_YURI` (merged into girls love by MAL - 404).
pub const GENRE_MANGA_YURI: i32 = 34;
/// `Constants::GENRE_MANGA_HAREM`.
pub const GENRE_MANGA_HAREM: i32 = 35;
/// `Constants::GENRE_MANGA_SLICE_OF_LIFE`.
pub const GENRE_MANGA_SLICE_OF_LIFE: i32 = 36;
/// `Constants::GENRE_MANGA_SUPERNATURAL`.
pub const GENRE_MANGA_SUPERNATURAL: i32 = 37;
/// `Constants::GENRE_MANGA_MILITARY`.
pub const GENRE_MANGA_MILITARY: i32 = 38;
/// `Constants::GENRE_MANGA_DETECTIVE`.
pub const GENRE_MANGA_DETECTIVE: i32 = 39;
/// `Constants::GENRE_MANGA_POLICE` (renamed to detective by MAL).
pub const GENRE_MANGA_POLICE: i32 = 39;
/// `Constants::GENRE_MANGA_PSYCHOLOGICAL`.
pub const GENRE_MANGA_PSYCHOLOGICAL: i32 = 40;
/// `Constants::GENRE_MANGA_SEINEN`.
pub const GENRE_MANGA_SEINEN: i32 = 41;
/// `Constants::GENRE_MANGA_JOSEI`.
pub const GENRE_MANGA_JOSEI: i32 = 42;
/// `Constants::GENRE_MANGA_DOUJINSHI` (removed by MAL - 404).
pub const GENRE_MANGA_DOUJINSHI: i32 = 43;
/// `Constants::GENRE_MANGA_CROSSDRESSING`.
pub const GENRE_MANGA_CROSSDRESSING: i32 = 44;
/// `Constants::GENRE_MANGA_GENDER_BENDER` (renamed to crossdressing by MAL).
pub const GENRE_MANGA_GENDER_BENDER: i32 = 44;
/// `Constants::GENRE_MANGA_SUSPENSE`.
pub const GENRE_MANGA_SUSPENSE: i32 = 45;
/// `Constants::GENRE_MANGA_THRILLER` (renamed to suspense by MAL).
pub const GENRE_MANGA_THRILLER: i32 = 45;
/// `Constants::GENRE_MANGA_AWARD_WINNING`.
pub const GENRE_MANGA_AWARD_WINNING: i32 = 46;
/// `Constants::GENRE_MANGA_GOURMET`.
pub const GENRE_MANGA_GOURMET: i32 = 47;
/// `Constants::GENRE_MANGA_WORKPLACE`.
pub const GENRE_MANGA_WORKPLACE: i32 = 48;
/// `Constants::GENRE_MANGA_WORK_LIFE` (renamed to workplace by MAL).
pub const GENRE_MANGA_WORK_LIFE: i32 = 48;
/// `Constants::GENRE_MANGA_EROTICA`.
pub const GENRE_MANGA_EROTICA: i32 = 49;
/// `Constants::GENRE_MANGA_ADULT_CAST`.
pub const GENRE_MANGA_ADULT_CAST: i32 = 50;
/// `Constants::GENRE_MANGA_ANTHROPOMORPHIC`.
pub const GENRE_MANGA_ANTHROPOMORPHIC: i32 = 51;
/// `Constants::GENRE_MANGA_CGDCT`.
pub const GENRE_MANGA_CGDCT: i32 = 52;
/// `Constants::GENRE_MANGA_CHILDCARE`.
pub const GENRE_MANGA_CHILDCARE: i32 = 53;
/// `Constants::GENRE_MANGA_COMBAT_SPORTS`.
pub const GENRE_MANGA_COMBAT_SPORTS: i32 = 54;
/// `Constants::GENRE_MANGA_DELINQUENTS`.
pub const GENRE_MANGA_DELINQUENTS: i32 = 55;
/// `Constants::GENRE_MANGA_EDUCATIONAL`.
pub const GENRE_MANGA_EDUCATIONAL: i32 = 56;
/// `Constants::GENRE_MANGA_GAG_HUMOR`.
pub const GENRE_MANGA_GAG_HUMOR: i32 = 57;
/// `Constants::GENRE_MANGA_GORE`.
pub const GENRE_MANGA_GORE: i32 = 58;
/// `Constants::GENRE_MANGA_HIGH_STAKES_GAME`.
pub const GENRE_MANGA_HIGH_STAKES_GAME: i32 = 59;
/// `Constants::GENRE_MANGA_IDOLS_FEMALE`.
pub const GENRE_MANGA_IDOLS_FEMALE: i32 = 60;
/// `Constants::GENRE_MANGA_IDOLS_MALE`.
pub const GENRE_MANGA_IDOLS_MALE: i32 = 61;
/// `Constants::GENRE_MANGA_ISEKAI`.
pub const GENRE_MANGA_ISEKAI: i32 = 62;
/// `Constants::GENRE_MANGA_IYASHIKEI`.
pub const GENRE_MANGA_IYASHIKEI: i32 = 63;
/// `Constants::GENRE_MANGA_LOVE_POLYGON`.
pub const GENRE_MANGA_LOVE_POLYGON: i32 = 64;
/// `Constants::GENRE_MANGA_MAGICAL_SEX_SHIFT`.
pub const GENRE_MANGA_MAGICAL_SEX_SHIFT: i32 = 65;
/// `Constants::GENRE_MANGA_MAHOU_SHOUJO`.
pub const GENRE_MANGA_MAHOU_SHOUJO: i32 = 66;
/// `Constants::GENRE_MANGA_MEDICAL`.
pub const GENRE_MANGA_MEDICAL: i32 = 67;
/// `Constants::GENRE_MANGA_MEMOIR`.
pub const GENRE_MANGA_MEMOIR: i32 = 68;
/// `Constants::GENRE_MANGA_ORGANIZED_CRIME`.
pub const GENRE_MANGA_ORGANIZED_CRIME: i32 = 69;
/// `Constants::GENRE_MANGA_OTAKU_CULTURE`.
pub const GENRE_MANGA_OTAKU_CULTURE: i32 = 70;
/// `Constants::GENRE_MANGA_PERFORMING_ARTS`.
pub const GENRE_MANGA_PERFORMING_ARTS: i32 = 71;
/// `Constants::GENRE_MANGA_PETS`.
pub const GENRE_MANGA_PETS: i32 = 72;
/// `Constants::GENRE_MANGA_REINCARNATION`.
pub const GENRE_MANGA_REINCARNATION: i32 = 73;
/// `Constants::GENRE_MANGA_REVERSE_HAREM`.
pub const GENRE_MANGA_REVERSE_HAREM: i32 = 74;
/// `Constants::GENRE_MANGA_ROMANTIC_SUBTEXT`.
pub const GENRE_MANGA_ROMANTIC_SUBTEXT: i32 = 75;
/// `Constants::GENRE_MANGA_SHOWBIZ`.
pub const GENRE_MANGA_SHOWBIZ: i32 = 76;
/// `Constants::GENRE_MANGA_SURVIVAL`.
pub const GENRE_MANGA_SURVIVAL: i32 = 77;
/// `Constants::GENRE_MANGA_TEAM_SPORTS`.
pub const GENRE_MANGA_TEAM_SPORTS: i32 = 78;
/// `Constants::GENRE_MANGA_TIME_TRAVEL`.
pub const GENRE_MANGA_TIME_TRAVEL: i32 = 79;
/// `Constants::GENRE_MANGA_VIDEO_GAME`.
pub const GENRE_MANGA_VIDEO_GAME: i32 = 80;
/// `Constants::GENRE_MANGA_VILLAINESS`.
pub const GENRE_MANGA_VILLAINESS: i32 = 81;
/// `Constants::GENRE_MANGA_VISUAL_ARTS`.
pub const GENRE_MANGA_VISUAL_ARTS: i32 = 82;

// ---------------------------------------------------------------------------
// User list status filters
// ---------------------------------------------------------------------------

/// `Constants::USER_ANIME_LIST_ALL`.
pub const USER_ANIME_LIST_ALL: i32 = 7;
/// `Constants::USER_ANIME_LIST_WATCHING`.
pub const USER_ANIME_LIST_WATCHING: i32 = 1;
/// `Constants::USER_ANIME_LIST_COMPLETED`.
pub const USER_ANIME_LIST_COMPLETED: i32 = 2;
/// `Constants::USER_ANIME_LIST_ONHOLD`.
pub const USER_ANIME_LIST_ONHOLD: i32 = 3;
/// `Constants::USER_ANIME_LIST_DROPPED`.
pub const USER_ANIME_LIST_DROPPED: i32 = 4;
/// `Constants::USER_ANIME_LIST_PTW`.
pub const USER_ANIME_LIST_PTW: i32 = 6;
/// `Constants::USER_ANIME_LIST_PLANTOWATCH` (alias of PTW).
pub const USER_ANIME_LIST_PLANTOWATCH: i32 = 6;

// ---------------------------------------------------------------------------
// User anime list order_by codes
// ---------------------------------------------------------------------------

/// `Constants::USER_ANIME_LIST_ORDER_BY_TITLE`.
pub const USER_ANIME_LIST_ORDER_BY_TITLE: i32 = 1;
/// `Constants::USER_ANIME_LIST_ORDER_BY_FINISHED_DATE`.
pub const USER_ANIME_LIST_ORDER_BY_FINISHED_DATE: i32 = 2;
/// `Constants::USER_ANIME_LIST_ORDER_BY_STARTED_DATE`.
pub const USER_ANIME_LIST_ORDER_BY_STARTED_DATE: i32 = 3;
/// `Constants::USER_ANIME_LIST_ORDER_BY_SCORE`.
pub const USER_ANIME_LIST_ORDER_BY_SCORE: i32 = 4;
/// `Constants::USER_ANIME_LIST_ORDER_BY_LAST_UPDATED`.
pub const USER_ANIME_LIST_ORDER_BY_LAST_UPDATED: i32 = 5;
/// `Constants::USER_ANIME_LIST_ORDER_BY_TYPE`.
pub const USER_ANIME_LIST_ORDER_BY_TYPE: i32 = 6;
/// `Constants::USER_ANIME_LIST_ORDER_BY_RATED`.
pub const USER_ANIME_LIST_ORDER_BY_RATED: i32 = 8;
/// `Constants::USER_ANIME_LIST_ORDER_BY_REWATCH_VALUE`.
pub const USER_ANIME_LIST_ORDER_BY_REWATCH_VALUE: i32 = 9;
/// `Constants::USER_ANIME_LIST_ORDER_BY_PRIORITY`.
pub const USER_ANIME_LIST_ORDER_BY_PRIORITY: i32 = 11;
/// `Constants::USER_ANIME_LIST_ORDER_BY_PROGRESS`.
pub const USER_ANIME_LIST_ORDER_BY_PROGRESS: i32 = 12;
/// `Constants::USER_ANIME_LIST_ORDER_BY_EPISODES` (alias of progress).
pub const USER_ANIME_LIST_ORDER_BY_EPISODES: i32 = 12;
/// `Constants::USER_ANIME_LIST_ORDER_BY_STORAGE`.
pub const USER_ANIME_LIST_ORDER_BY_STORAGE: i32 = 13;
/// `Constants::USER_ANIME_LIST_ORDER_BY_AIR_START`.
pub const USER_ANIME_LIST_ORDER_BY_AIR_START: i32 = 14;
/// `Constants::USER_ANIME_LIST_ORDER_BY_AIR_END`.
pub const USER_ANIME_LIST_ORDER_BY_AIR_END: i32 = 15;
/// `Constants::USER_ANIME_LIST_ORDER_BY_STATUS`.
pub const USER_ANIME_LIST_ORDER_BY_STATUS: i32 = 16;

/// `Constants::USER_ANIME_LIST_CURRENTLY_AIRING`.
pub const USER_ANIME_LIST_CURRENTLY_AIRING: i32 = 1;
/// `Constants::USER_ANIME_LIST_FINISHED_AIRING`.
pub const USER_ANIME_LIST_FINISHED_AIRING: i32 = 2;
/// `Constants::USER_ANIME_LIST_NOT_YET_AIRED`.
pub const USER_ANIME_LIST_NOT_YET_AIRED: i32 = 3;

/// `Constants::USER_MANGA_LIST_ALL`.
pub const USER_MANGA_LIST_ALL: i32 = 7;
/// `Constants::USER_MANGA_LIST_READING`.
pub const USER_MANGA_LIST_READING: i32 = 1;
/// `Constants::USER_MANGA_LIST_COMPLETED`.
pub const USER_MANGA_LIST_COMPLETED: i32 = 2;
/// `Constants::USER_MANGA_LIST_ONHOLD`.
pub const USER_MANGA_LIST_ONHOLD: i32 = 3;
/// `Constants::USER_MANGA_LIST_DROPPED`.
pub const USER_MANGA_LIST_DROPPED: i32 = 4;
/// `Constants::USER_MANGA_LIST_PTR`.
pub const USER_MANGA_LIST_PTR: i32 = 6;
/// `Constants::USER_MANGA_LIST_PLANTOREAD` (alias of PTR).
pub const USER_MANGA_LIST_PLANTOREAD: i32 = 6;

// ---------------------------------------------------------------------------
// User manga list order_by codes
// ---------------------------------------------------------------------------

/// `Constants::USER_MANGA_LIST_ORDER_BY_TITLE`.
pub const USER_MANGA_LIST_ORDER_BY_TITLE: i32 = 1;
/// `Constants::USER_MANGA_LIST_ORDER_BY_FINISHED_DATE`.
pub const USER_MANGA_LIST_ORDER_BY_FINISHED_DATE: i32 = 2;
/// `Constants::USER_MANGA_LIST_ORDER_BY_STARTED_DATE`.
pub const USER_MANGA_LIST_ORDER_BY_STARTED_DATE: i32 = 3;
/// `Constants::USER_MANGA_LIST_ORDER_BY_SCORE`.
pub const USER_MANGA_LIST_ORDER_BY_SCORE: i32 = 4;
/// `Constants::USER_MANGA_LIST_ORDER_BY_LAST_UPDATED`.
pub const USER_MANGA_LIST_ORDER_BY_LAST_UPDATED: i32 = 5;
/// `Constants::USER_MANGA_LIST_ORDER_BY_PRIORITY`.
pub const USER_MANGA_LIST_ORDER_BY_PRIORITY: i32 = 8;
/// `Constants::USER_MANGA_LIST_ORDER_BY_CHAPTERS`.
pub const USER_MANGA_LIST_ORDER_BY_CHAPTERS: i32 = 9;
/// `Constants::USER_MANGA_LIST_ORDER_BY_VOLUMES`.
pub const USER_MANGA_LIST_ORDER_BY_VOLUMES: i32 = 10;
/// `Constants::USER_MANGA_LIST_ORDER_BY_TYPE`.
pub const USER_MANGA_LIST_ORDER_BY_TYPE: i32 = 11;
/// `Constants::USER_MANGA_LIST_ORDER_BY_PUBLISH_START`.
pub const USER_MANGA_LIST_ORDER_BY_PUBLISH_START: i32 = 12;
/// `Constants::USER_MANGA_LIST_ORDER_BY_PUBLISH_END`.
pub const USER_MANGA_LIST_ORDER_BY_PUBLISH_END: i32 = 13;
/// `Constants::USER_MANGA_LIST_ORDER_BY_STATUS`.
pub const USER_MANGA_LIST_ORDER_BY_STATUS: i32 = 14;

/// `Constants::USER_MANGA_LIST_CURRENTLY_PUBLISHING`.
pub const USER_MANGA_LIST_CURRENTLY_PUBLISHING: i32 = 1;
/// `Constants::USER_MANGA_LIST_FINISHED_PUBLISHING`.
pub const USER_MANGA_LIST_FINISHED_PUBLISHING: i32 = 2;
/// `Constants::USER_MANGA_LIST_NOT_YET_PUBLISHED`.
pub const USER_MANGA_LIST_NOT_YET_PUBLISHED: i32 = 3;
/// `Constants::USER_MANGA_LIST_ON_HIATUS`.
pub const USER_MANGA_LIST_ON_HIATUS: i32 = 4;
/// `Constants::USER_MANGA_LIST_DISCONTINUED`.
pub const USER_MANGA_LIST_DISCONTINUED: i32 = 5;

/// `Constants::USER_LIST_SORT_DESCENDING`.
pub const USER_LIST_SORT_DESCENDING: i32 = 1;
/// `Constants::USER_LIST_SORT_ASCENDING`.
pub const USER_LIST_SORT_ASCENDING: i32 = -1;

/// `Constants::REVIEWS_SORT_MOST_VOTED`.
pub const REVIEWS_SORT_MOST_VOTED: &str = "mostvoted";
/// `Constants::REVIEWS_SORT_OLDEST`.
pub const REVIEWS_SORT_OLDEST: &str = "oldest";
/// `Constants::REVIEWS_SORT_NEWEST`.
pub const REVIEWS_SORT_NEWEST: &str = "newest";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_seasons() {
        assert_eq!(BASE_URL, "https://myanimelist.net");
        assert_eq!(CDN_URL, "https://cdn.myanimelist.net");
        assert_eq!(SEASONS, ["Winter", "Spring", "Summer", "Fall"]);
        assert_eq!(
            [WINTER, SPRING, SUMMER, FALL],
            ["winter", "spring", "summer", "fall"]
        );
    }

    #[test]
    fn anime_genre_ids_match_constants_php() {
        assert_eq!(GENRE_ANIME_ACTION, 1);
        assert_eq!(GENRE_ANIME_HENTAI, 12);
        assert_eq!(GENRE_ANIME_EROTICA, 49);
        assert_eq!(GENRE_ANIME_MAGIC, 16);
        assert_eq!(GENRE_ANIME_AWARD_WINNING, 46);
        assert_eq!(GENRE_ANIME_CROSSDRESSING, 81);
        // MAL renames kept as aliases.
        assert_eq!(GENRE_ANIME_CARS, GENRE_ANIME_RACING);
        assert_eq!(GENRE_ANIME_DEMENTIA, GENRE_ANIME_AVANT_GARDE);
        assert_eq!(GENRE_ANIME_DEMONS, GENRE_ANIME_MYTHOLOGY);
        assert_eq!(GENRE_ANIME_GAME, GENRE_ANIME_STRATEGY_GAME);
        assert_eq!(GENRE_ANIME_POLICE, GENRE_ANIME_DETECTIVE);
        assert_eq!(GENRE_ANIME_THRILLER, GENRE_ANIME_SUSPENSE);
        assert_eq!(GENRE_ANIME_WORK_LIFE, GENRE_ANIME_WORKPLACE);
        assert_eq!(GENRE_ANIME_SHOUJO_AI, GENRE_ANIME_GIRLS_LOVE);
        assert_eq!(GENRE_ANIME_SHOUNEN_AI, GENRE_ANIME_BOYS_LOVE);
        assert_eq!(GENRE_ANIME_YAOI, 33);
        assert_eq!(GENRE_ANIME_YURI, 34);
    }

    #[test]
    fn manga_genre_ids_match_constants_php() {
        assert_eq!(GENRE_MANGA_ACTION, 1);
        assert_eq!(GENRE_MANGA_HENTAI, 12);
        assert_eq!(GENRE_MANGA_EROTICA, 49);
        assert_eq!(GENRE_MANGA_SEINEN, 41);
        assert_eq!(GENRE_MANGA_JOSEI, 42);
        assert_eq!(GENRE_MANGA_DOUJINSHI, 43);
        assert_eq!(GENRE_MANGA_MEMOIR, 68);
        assert_eq!(GENRE_MANGA_VILLAINESS, 81);
        assert_eq!(GENRE_MANGA_VISUAL_ARTS, 82);
        assert_eq!(GENRE_MANGA_GENDER_BENDER, GENRE_MANGA_CROSSDRESSING);
        assert_eq!(GENRE_MANGA_SHOUJO_AI, GENRE_MANGA_GIRLS_LOVE);
        assert_eq!(GENRE_MANGA_SHOUNEN_AI, GENRE_MANGA_BOYS_LOVE);
        assert_eq!(GENRE_MANGA_WORK_LIFE, GENRE_MANGA_WORKPLACE);
        assert_eq!(GENRE_MANGA_THRILLER, GENRE_MANGA_SUSPENSE);
    }

    #[test]
    fn search_and_user_list_codes_match_constants_php() {
        assert_eq!(SEARCH_ANIME_TV, 1);
        assert_eq!(SEARCH_ANIME_MUSIC, 6);
        assert_eq!(SEARCH_MANGA_MANHUA, 6);
        assert_eq!(
            SEARCH_ANIME_STATUS_COMPLETED,
            SEARCH_ANIME_STATUS_FINISHED_AIRING
        );
        assert_eq!(SEARCH_ANIME_STATUS_TBA, SEARCH_ANIME_STATUS_TO_BE_AIRD);
        assert_eq!(
            SEARCH_MANGA_STATUS_COMPLETED,
            SEARCH_MANGA_STATUS_FINISHED_PUBLISHING
        );
        assert_eq!(SEARCH_MANGA_STATUS_TBP, SEARCH_MANGA_STATUS_TO_BE_PUBLISHED);
        assert_eq!(SEARCH_ANIME_RATING_ALL, SEARCH_ANIME_RATING_G);
        assert_eq!(SEARCH_ANIME_RATING_HENTAI, SEARCH_ANIME_RATING_RX);
        assert_eq!(SEARCH_USER_GENDER_ANY, -1);
        assert_eq!(SEARCH_USER_GENDER_NONBINARY, 3);
        assert_eq!(SEARCH_SORT_ASCENDING, 0);

        assert_eq!(USER_ANIME_LIST_ALL, 7);
        assert_eq!(USER_ANIME_LIST_PLANTOWATCH, USER_ANIME_LIST_PTW);
        assert_eq!(USER_ANIME_LIST_ORDER_BY_TITLE, 1);
        assert_eq!(USER_ANIME_LIST_ORDER_BY_LAST_UPDATED, 5);
        assert_eq!(USER_ANIME_LIST_ORDER_BY_PRIORITY, 11);
        assert_eq!(USER_ANIME_LIST_ORDER_BY_STATUS, 16);
        assert_eq!(
            USER_ANIME_LIST_ORDER_BY_EPISODES,
            USER_ANIME_LIST_ORDER_BY_PROGRESS
        );
        assert_eq!(USER_MANGA_LIST_PLANTOREAD, USER_MANGA_LIST_PTR);
        assert_eq!(USER_MANGA_LIST_ORDER_BY_CHAPTERS, 9);
        assert_eq!(USER_MANGA_LIST_ORDER_BY_STATUS, 14);
        assert_eq!(USER_LIST_SORT_ASCENDING, -1);
        assert_eq!(USER_LIST_SORT_DESCENDING, 1);

        assert_eq!(REVIEWS_SORT_MOST_VOTED, "mostvoted");
        assert_eq!(REVIEWS_SORT_OLDEST, "oldest");
        assert_eq!(REVIEWS_SORT_NEWEST, "newest");
    }
}
