package main

import (
	"encoding/csv"
	"fmt"
	"image/color"
	"log"
	"os"
	"sort"
	"strconv"

	"gonum.org/v1/plot"
	"gonum.org/v1/plot/plotter"
	"gonum.org/v1/plot/vg"
	"gonum.org/v1/plot/vg/draw"
)

const (
	ColMode   = 0
	ColRPS    = 1
	ColLoss   = 5
	ColMeanMS = 7
	ColMaxMS  = 11
)

func main() {
	fileName := "../results.csv"
	records, err := readCSV(fileName)
	if err != nil {
		log.Fatal(err)
	}

	createPlot("loss.png", "Loss Rate (%)", "RPS", "Loss (%)", records, ColLoss)
	createPlot("mean.png", "Mean Latency (ms)", "RPS", "Time (ms)", records, ColMeanMS)
	createPlot("max.png", "Max Latency (ms)", "RPS", "Time (ms)", records, ColMaxMS)

}

type tempStat struct {
	sum   float64
	count int
}

func createPlot(filename, title, xLabel, yLabel string, records [][]string, valCol int) {
	p := plot.New()

	p.Title.Text = title
	p.X.Label.Text = xLabel
	p.Y.Label.Text = yLabel

	p.Title.TextStyle.Font.Size = vg.Points(20)
	p.X.Label.TextStyle.Font.Size = vg.Points(14)
	p.Y.Label.TextStyle.Font.Size = vg.Points(14)
	p.Legend.TextStyle.Font.Size = vg.Points(12)
	p.Legend.Top = true
	p.Legend.Padding = vg.Points(5)

	p.Add(plotter.NewGrid())

	aggregationMap := make(map[string]map[float64]*tempStat)

	for i, row := range records {
		if i == 0 {
			continue
		}

		mode := row[ColMode]
		rps, err1 := strconv.ParseFloat(row[ColRPS], 64)
		val, err2 := strconv.ParseFloat(row[valCol], 64)

		if err1 != nil || err2 != nil {
			continue
		}

		if _, ok := aggregationMap[mode]; !ok {
			aggregationMap[mode] = make(map[float64]*tempStat)
		}
		if _, ok := aggregationMap[mode][rps]; !ok {
			aggregationMap[mode][rps] = &tempStat{}
		}

		aggregationMap[mode][rps].sum += val
		aggregationMap[mode][rps].count++
	}

	dataByMode := make(map[string]plotter.XYs)

	for mode, rpsMap := range aggregationMap {
		var points plotter.XYs
		for rps, stat := range rpsMap {
			avg := stat.sum / float64(stat.count)
			points = append(points, plotter.XY{X: rps, Y: avg})
		}

		sort.Slice(points, func(i, j int) bool {
			return points[i].X < points[j].X
		})

		dataByMode[mode] = points
	}

	colors := []color.Color{
		color.RGBA{R: 255, G: 0, B: 0, A: 255},
		color.RGBA{R: 0, G: 0, B: 255, A: 255},
		color.RGBA{R: 0, G: 128, B: 0, A: 255},
		color.RGBA{R: 255, G: 165, B: 0, A: 255},
		color.RGBA{R: 128, G: 0, B: 128, A: 255},
		color.RGBA{R: 0, G: 255, B: 255, A: 255},
		color.RGBA{R: 165, G: 42, B: 42, A: 255},
		color.RGBA{R: 0, G: 0, B: 0, A: 255},
		color.RGBA{R: 255, G: 0, B: 255, A: 255},
		color.RGBA{R: 128, G: 128, B: 128, A: 255},
	}

	var sortedModes []string
	for mode := range dataByMode {
		sortedModes = append(sortedModes, mode)
	}
	sort.Strings(sortedModes)

	for i, mode := range sortedModes {
		xys := dataByMode[mode]

		line, points, err := plotter.NewLinePoints(xys)
		if err != nil {
			log.Panic(err)
		}

		c := colors[i%len(colors)]
		line.Color = c

		if mode == "longpull-direct" || mode == "webhook-direct" {
			line.Width = vg.Points(6)
		} else {
			line.Width = vg.Points(2)
		}

		points.Shape = draw.CircleGlyph{}
		points.Color = c
		points.Radius = vg.Points(3)

		p.Add(line, points)
		p.Legend.Add(mode, line, points)
	}

	if err := p.Save(12*vg.Inch, 8*vg.Inch, filename); err != nil {
		log.Panic(err)
	}
}

func readCSV(path string) ([][]string, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, fmt.Errorf("cannot open %s: %v", path, err)
	}
	defer file.Close()

	reader := csv.NewReader(file)
	return reader.ReadAll()
}
