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
  #let (n, sn, order_id, qty, name, addr) = row
  #let n = int(n)

  #place(top + left)[
    #image("demo_pack/frame-1.pdf")
  ]

  #let fields = (
    ("Order ID", order_id),
    ("Price", "168 RMB"),
    ("Quantity", qty),
    ("From", "上海"),
    ("Ship to", name),
    ("Address", addr),
    ("SN", sn),
  )

  #for (i, (label, value)) in fields.enumerate() {
    place-box(250pt, 1872.5pt + i * 20.98pt, anchor: bottom + right)[
      #text(size: 16pt, style: "italic", tracking: -0.5pt)[#label]
    ]
    place-box(252pt, 1872pt + i * 20.98pt, anchor: bottom + left)[
      #text(size: 16pt)[:]
    ]
    place-box(271pt, 1872.5pt + i * 20.98pt, anchor: bottom + left)[ #text(size: 16pt)[#value]
    ]
  }

  #place(top + left, dx: 0pt, dy: 174pt)[
    #image("demo_pack/images/a" + str(calc.rem(n, 9) + 1) + ".png")
  ]
  #place(top + left, dx: 545pt, dy: 1842pt)[
    #image("demo_pack/images/b" + str(calc.rem(n, 5) + 1) + ".png")
  ]
  
  #pagebreak()
]
