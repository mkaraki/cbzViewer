package comic_info

type ComicInfo struct {
	Title               string               `xml:"Title"`
	Series              string               `xml:"Series"`
	Number              string               `xml:"Number"`
	Count               int                  `xml:"Count"`
	Volume              int                  `xml:"Volume"`
	AlternateSeries     string               `xml:"AlternateSeries"`
	AlternateNumber     string               `xml:"AlternateNumber"`
	AlternateCount      int                  `xml:"AlternateCount"`
	Summary             string               `xml:"Summary"`
	Notes               string               `xml:"Notes"`
	Year                int                  `xml:"Year"`
	Month               int                  `xml:"Month"`
	Day                 int                  `xml:"Day"`
	Writer              string               `xml:"Writer"`
	Penciller           string               `xml:"Penciller"`
	Inker               string               `xml:"Inker"`
	Colorist            string               `xml:"Colorist"`
	Letterer            string               `xml:"Letterer"`
	CoverArtist         string               `xml:"CoverArtist"`
	Editor              string               `xml:"Editor"`
	Publisher           string               `xml:"Publisher"`
	Imprint             string               `xml:"Imprint"`
	Genre               string               `xml:"Genre"`
	Web                 string               `xml:"Web"`
	PageCount           int                  `xml:"PageCount"`
	LanguageISO         string               `xml:"LanguageISO"`
	Format              string               `xml:"Format"`
	BlackAndWhite       YesNo                `xml:"BlackAndWhite"`
	Manga               MangaType            `xml:"Manga"`
	Characters          string               `xml:"Characters"`
	Teams               string               `xml:"Teams"`
	Locations           string               `xml:"Locations"`
	ScanInformation     string               `xml:"ScanInformation"`
	StoryArc            string               `xml:"StoryArc"`
	SeriesGroup         string               `xml:"SeriesGroup"`
	AgeRating           AgeRatingType        `xml:"AgeRating"`
	Pages               ArrayOfComicPageInfo `xml:"Pages"`
	CommunityRating     Rating               `xml:"CommunityRating"`
	MainCharacterOrTeam string               `xml:"MainCharacterOrTeam"`
	Review              string               `xml:"Review"`
}

type YesNo string

const (
	YesNoYes     YesNo = "Yes"
	YesNoNo      YesNo = "No"
	YesNoUnknown YesNo = "Unknown"
)

type MangaType string

const (
	MangaUnknown           MangaType = "Unknown"
	MangaNo                MangaType = "No"
	MangaYes               MangaType = "Yes"
	MangaYesAndRightToLeft MangaType = "YesAndRightToLeft"
)

type Rating int

type AgeRatingType string

const (
	AgeRatingUnknown          AgeRatingType = "Unknown"
	AgeRatingAdultsOnly18Plus AgeRatingType = "Adults Only 18+"
	AgeRatingEarlyChildhood   AgeRatingType = "Early Childhood"
	AgeRatingEveryone         AgeRatingType = "Everyone"
	AgeRatingEveryone10Plus   AgeRatingType = "Everyone 10+"
	AgeRatingG                AgeRatingType = "G"
	AgeRatingKidsToAdults     AgeRatingType = "Kids to Adults"
	AgeRatingM                AgeRatingType = "M"
	AgeRatingMA15Plus         AgeRatingType = "MA15+"
	AgeRatingMature17Plus     AgeRatingType = "Mature 17+"
	AgeRatingPG               AgeRatingType = "PG"
	AgeRatingR18Plus          AgeRatingType = "R18+"
	AgeRatingRatingPending    AgeRatingType = "Rating Pending"
	AgeRatingTeen             AgeRatingType = "Teen"
	AgeRatingX18Plus          AgeRatingType = "X18+"
)

type ArrayOfComicPageInfo struct {
	Page []*ComicPageInfo `xml:"Page"`
}

type ComicPageInfo struct {
	Image       int           `xml:"Image,attr"`
	Type        ComicPageType `xml:"Type,attr,omitempty"`
	DoublePage  bool          `xml:"DoublePage,attr,omitempty"`
	ImageSize   int64         `xml:"ImageSize,attr,omitempty"`
	Key         string        `xml:"Key,attr,omitempty"`
	Bookmark    string        `xml:"Bookmark,attr,omitempty"`
	ImageWidth  int           `xml:"ImageWidth,attr,omitempty"`
	ImageHeight int           `xml:"ImageHeight,attr,omitempty"`
}

type ComicPageType string

const (
	ComicPageTypeFrontCover    ComicPageType = "FrontCover"
	ComicPageTypeInnerCover    ComicPageType = "InnerCover"
	ComicPageTypeRoundup       ComicPageType = "Roundup"
	ComicPageTypeStory         ComicPageType = "Story"
	ComicPageTypeAdvertisement ComicPageType = "Advertisement"
	ComicPageTypeEditorial     ComicPageType = "Editorial"
	ComicPageTypeLetters       ComicPageType = "Letters"
	ComicPageTypePreview       ComicPageType = "Preview"
	ComicPageTypeBackCover     ComicPageType = "BackCover"
	ComicPageTypeOther         ComicPageType = "Other"
	ComicPageTypeDeleted       ComicPageType = "Deleted"
)
