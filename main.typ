// cat main.typ | SOURCE_DATE_EPOCH=0 typst compile --root demo_pack --font-path demo_pack - /tmp/1.pdf

#let place-text(
  x, y, width: 100%, height: 100%, anchor: top + left,
  font: none, size: none, style: "normal", tracking: 0pt,
  body,
) = {
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
    #box(width: width, height: height)[
      #align(anchor, text(font: font, size: size, style: style, tracking: tracking)[#body])
    ]
  ]
}

#set page(width: 759pt, height: 2663pt, margin: 0pt)

#for row in csv("list.csv", row-type: dictionary) [
  #let n = int(row.n)

  #place(top + left, dx: 0pt, dy: 174pt)[
    #image("a" + str(calc.rem(n, 9) + 1) + ".png")
  ]

  #place(top + left)[
    #image("frame.pdf")
  ]

  #let fields = (
    ("Order ID", row.order_id),
    ("Price", "168 RMB"),
    ("Quantity", row.qty),
    ("From", "上海"),
    ("Ship to", row.name),
    ("Address", row.addr),
    ("SN", row.sn),
  )

  #for (i, (label, value)) in fields.enumerate() {
    place-text(250pt, 1872.5pt + i * 20.98pt, anchor: bottom + right,
      font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt,
      style: "italic", tracking: -0.5pt)[#label]
    place-text(252pt, 1872pt + i * 20.98pt, anchor: bottom + left,
      font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt,
      )[:]
    place-text(271pt, 1872.5pt + i * 20.98pt, anchor: bottom + left,
      font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt,
      )[#value]
  }

  #place(top + left, dx: 545pt, dy: 1842pt)[
    #image("b" + str(calc.rem(n, 5) + 1) + ".png")
  ]
  
  #pagebreak(weak: true)
]
