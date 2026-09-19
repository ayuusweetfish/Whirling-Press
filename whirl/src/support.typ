#let place-element(
  x, y, width: auto, height: auto, anchor: top + left,
  rotate: 0deg, scale-x: 100%, scale-y: 100%, skew-x: 0deg, skew-y: 0deg,
  debug: false,
  in_flow: false,
  body,
) = context {
  let (width, height) = if width != auto and height != auto {
    (width, height)
  } else {
    let dims = measure(width: width, height: height, body)
    (
      if width == auto { dims.width } else { width },
      if height == auto { dims.height } else { height },
    )
  }

  let dx = x - if anchor.x == right { width } else if anchor.x == center { width / 2 } else { 0pt }
  let dy = y - if anchor.y == bottom { height } else if anchor.y == horizon { height / 2 } else { 0pt }

  let content = [
    #std.rotate(rotate, origin: anchor)[
      #skew(ax: skew-x, ay: skew-y, origin: anchor)[
        #std.scale(x: scale-x, y: scale-y, origin: anchor)[
          #box(width: width, height: height)[
            #align(anchor, body)
            #if debug {
              place(top + left, dx: 0pt, dy: 0pt, rect(width: 100%, height: 100%))
            }
          ]
        ]
      ]
    ]
  ]

  if in_flow { content }
  else { place(top + left, dx: dx, dy: dy, content) }

  if debug {
    place(top + left, dx: x - 2pt, dy: y - 2pt)[
      #circle(radius: 2pt, fill: none, stroke: 1pt + black)
    ]
  }
}
