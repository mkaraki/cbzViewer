/*
 * Based on The Anansi Project's ComicInfo Scheme version 2.0
 *
 * MIT License
 *
 * Copyright (c) 2021 anansi-project
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

package comic_info

type ComicInfo struct {
	Title               string               `xml:"Title"`           // Title of the book
	Series              string               `xml:"Series"`          // Series name of the book
	Number              string               `xml:"Number"`          // Number of the book in series
	Count               int                  `xml:"Count"`           // Total number of books in the series
	Volume              int                  `xml:"Volume"`          // Volume containing the book. This may number (e.g. 1, 2, 3) or year (e.g. 2020).
	AlternateSeries     string               `xml:"AlternateSeries"` // Series name of the book for cross-over
	AlternateNumber     string               `xml:"AlternateNumber"` // Number of the book in series for cross-over
	AlternateCount      int                  `xml:"AlternateCount"`  // Total number of books in series for cross-over
	Summary             string               `xml:"Summary"`         // Description or summary of book
	Notes               string               `xml:"Notes"`           // Free text field
	Year                int                  `xml:"Year"`            // Release date
	Month               int                  `xml:"Month"`           // Release date
	Day                 int                  `xml:"Day"`             // Release date
	Writer              string               `xml:"Writer"`          // Responsible person/organization for scenario writing. Comma separated.
	Penciller           string               `xml:"Penciller"`       // Responsible person/organization for draw art. Comma separated.
	Inker               string               `xml:"Inker"`           // Responsible person/organization for inking the pencil art. Comma separated.
	Colorist            string               `xml:"Colorist"`        // Responsible person/organization for drawing applying color for drawing. Comma separated.
	Letterer            string               `xml:"Letterer"`        // Responsible person/organization for draw text/speech bubble. Comma separated.
	CoverArtist         string               `xml:"CoverArtist"`     // Responsible person/organization for drawing the cover art.
	Editor              string               `xml:"Editor"`          // Responsible person/organization for editing
	Publisher           string               `xml:"Publisher"`       // Responsible person/organization for publish/release/issue the resource.
	Imprint             string               `xml:"Imprint"`         // Imprint is a group that belong larger imprint or publisher.
	Genre               string               `xml:"Genre"`           // Genre of books. Comma separated.
	Web                 string               `xml:"Web"`             // Reference site's url. Comma separated. Should be uri encoded.
	PageCount           int                  `xml:"PageCount"`       // Number of pages
	LanguageISO         string               `xml:"LanguageISO"`     // Language of book. Value should `IETF BCP 47 language tag`.
	Format              string               `xml:"Format"`
	BlackAndWhite       YesNo                `xml:"BlackAndWhite"`       // Is black and white book or not
	Manga               MangaType            `xml:"Manga"`               // Is manga or not and RightToLeft or not.
	Characters          string               `xml:"Characters"`          // Characters present in the book. Comma separated.
	Teams               string               `xml:"Teams"`               // Teams present in the book. Comma separated.
	Locations           string               `xml:"Locations"`           // Locations mentioned in the book. Comma separated.
	ScanInformation     string               `xml:"ScanInformation"`     // Free text about Scan Information
	StoryArc            string               `xml:"StoryArc"`            // Story arc of books belongs to
	SeriesGroup         string               `xml:"SeriesGroup"`         // Group or collection that series belongs to. Comma separated
	AgeRating           AgeRatingType        `xml:"AgeRating"`           // Age rating of book.
	Pages               ArrayOfComicPageInfo `xml:"Pages"`               // Page info
	CommunityRating     Rating               `xml:"CommunityRating"`     // Community rating of book from 0.00 to 5.00
	MainCharacterOrTeam string               `xml:"MainCharacterOrTeam"` // Main character or team name. This field only can store one character/team.
	Review              string               `xml:"Review"`              // Review of the book
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

// Rating for work.
// Originally, this field is declare as decimal (with 2 fraction digits, e.g. 1.25, 3.00, 4.95)
// Minimal 0.00, Maximum 5.00.
type Rating float32

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
	Image       int           `xml:"Image,attr"` // Page number
	Type        ComicPageType `xml:"Type,attr,omitempty"`
	DoublePage  bool          `xml:"DoublePage,attr,omitempty"` // Is double spread page or not
	ImageSize   int64         `xml:"ImageSize,attr,omitempty"`  // Image size. bytes.
	Key         string        `xml:"Key,attr,omitempty"`
	Bookmark    string        `xml:"Bookmark,attr,omitempty"`    // Reserved for specific application to bookmark page.
	ImageWidth  int           `xml:"ImageWidth,attr,omitempty"`  // Width of image file
	ImageHeight int           `xml:"ImageHeight,attr,omitempty"` // height of image file
}

type ComicPageType string

const (
	ComicPageTypeFrontCover    ComicPageType = "FrontCover" // Front Cover of book
	ComicPageTypeInnerCover    ComicPageType = "InnerCover" // Sometimes, there are second cover inside the book. Use this for that kinds of page.
	ComicPageTypeRoundup       ComicPageType = "Roundup"    // Summary of previous issue
	ComicPageTypeStory         ComicPageType = "Story"
	ComicPageTypeAdvertisement ComicPageType = "Advertisement"
	ComicPageTypeEditorial     ComicPageType = "Editorial"
	ComicPageTypeLetters       ComicPageType = "Letters" // Fan letters
	ComicPageTypePreview       ComicPageType = "Preview" // trailer/preview of the next book or another book
	ComicPageTypeBackCover     ComicPageType = "BackCover"
	ComicPageTypeOther         ComicPageType = "Other"
	ComicPageTypeDeleted       ComicPageType = "Deleted" // To indicate that this page should omit in viewer
)
