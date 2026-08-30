// cat main.typ | SOURCE_DATE_EPOCH=0 typst compile --root demo_pack --font-path demo_pack - /tmp/1.pdf

#let place-element(
  x, y, width: 100%, height: 100%, anchor: top + left,
  rotate: 0deg, scale-x: 100%, scale-y: 100%, skew-x: 0deg, skew-y: 0deg,
  body,
) = {
  let dx = x - if anchor.x == right { width } else if anchor.x == center { width / 2 } else { 0pt }
  let dy = y - if anchor.y == bottom { height } else if anchor.y == horizon { height / 2 } else { 0pt }

  place(top + left, dx: dx, dy: dy)[
    #std.rotate(rotate, origin: anchor)[
      #scale(x: scale-x, y: scale-y, origin: anchor)[
        #skew(ax: skew-x, ay: skew-y, origin: anchor)[
          #box(width: width, height: height)[
            #align(anchor, body)
          ]
        ]
      ]
    ]
  ]
}

#set page(width: 759pt, height: 2663pt, margin: 0pt)

#for row in csv("list.csv", row-type: dictionary) [
  #place-element(0pt, 174pt)[#image(row.img_a)]

  #place-element(0pt, 0pt)[#image("frame.pdf")]

  #place-element(250pt, 1872.50pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[Order ID]]
  #place-element(252pt, 1872.00pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1872.50pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[#row.order_id]]

  #place-element(250pt, 1893.48pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[Price]]
  #place-element(252pt, 1892.98pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1893.48pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[168 RMB]]

  #place-element(250pt, 1914.46pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[Quantity]]
  #place-element(252pt, 1913.96pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1914.46pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[#row.qty]]

  #place-element(250pt, 1935.44pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[From]]
  #place-element(252pt, 1934.94pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1935.44pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[上海]]

  #place-element(250pt, 1956.42pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[Ship to]]
  #place-element(252pt, 1955.92pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1956.42pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[#row.name]]

  #place-element(250pt, 1977.40pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[Address]]
  #place-element(252pt, 1976.90pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1977.40pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[#row.addr]]

  #place-element(250pt, 1998.38pt, anchor: bottom + right)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt, style: "italic", tracking: -0.5pt)[SN]]
  #place-element(252pt, 1997.88pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[:]]
  #place-element(271pt, 1998.38pt, anchor: bottom + left)[#text(font: ("Instrument Sans", "Noto Sans CJK SC"), size: 16pt)[#row.sn]]

  #place-element(545pt, 1842pt)[#image(row.img_b)]
  
  #pagebreak(weak: true)
]
