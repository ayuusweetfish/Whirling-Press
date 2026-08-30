// SOURCE_DATE_EPOCH=0 typst compile --font-path demo_pack/fonts % /tmp/1.pdf

#let place-box(x, y, width: 100%, height: 100%, anchor: top + left, body) = {
  let dx = x
  let dy = y

  if anchor.x == right {
    dx -= width
  } else if anchor.x == center {
    dx -= width / 2
  }

  if anchor.y == bottom {
    dy -= height
  } else if anchor.y == horizon {
    dy -= height / 2
  }

  place(top + left, dx: dx, dy: dy)[
    #box(width: width, height: height, align(anchor, body))
  ]
}

#set page(width: 759pt, height: 2663pt, margin: 0pt)

#let data = csv("demo_pack/list.tsv", delimiter: "\t")

#set text(font: ("Instrument Sans", "Noto Sans CJK SC"))

#for row in data [
  #place(top + left)[
    #image("demo_pack/frame-1.pdf")
  ]

  #place-box(271pt, 1872.5pt, anchor: bottom + left)[
    #text(size: 16pt)[#row.at(3)]
  ]

  #place(top + left, dx: 0pt, dy: 174pt)[
    #image("demo_pack/images/a1.png")
  ]
  
  #pagebreak()
]
