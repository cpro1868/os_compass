package progress

import (
	"bytes"
	"strings"
	"testing"
)

func TestRendererShowsDockerPullStyleLines(t *testing.T) {
	var output bytes.Buffer
	renderer := NewRenderer(&output)

	renderer.EventName("cover.png", "Downloading")
	renderer.Event(Event{Name: "clip.webm", Status: "Complete"})

	lines := strings.Split(output.String(), "\n")
	if len(lines) < 3 {
		t.Fatalf("expected at least two output lines, got %q", output.String())
	}

	if lines[0] != "cover.png: Downloading" {
		t.Fatalf("expected first line to show download status, got %q", lines[0])
	}
	if lines[1] != "clip.webm: Complete" {
		t.Fatalf("expected second line to show complete status, got %q", lines[1])
	}
}

func TestNewRendererWithNilWriterDoesNotPanic(t *testing.T) {
	renderer := NewRenderer(nil)

	renderer.EventName("cover.png", "Downloading")
	renderer.Event(Event{Name: "cover.png", Status: "Complete"})
}
