from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

def draw_rounded_rect(pen, x0, y0, x1, y1, r, clockwise=True):
  corners = [
    ((x0, y1), (x0, y1 - r), (x0 + r, y1)),
    ((x1, y1), (x1 - r, y1), (x1, y1 - r)),
    ((x1, y0), (x1, y0 + r), (x1 - r, y0)),
    ((x0, y0), (x0 + r, y0), (x0, y0 + r)),
  ]
  if not clockwise:
    corners = [(c, b, a) for c, a, b in reversed(corners)]
  pen.moveTo(corners[-1][2])
  for ctrl, entry, exit in corners:
    pen.lineTo(entry)
    pen.qCurveTo(ctrl, exit)
  pen.closePath()

def draw_glyph(pen, font, char, x0, y0, x1, y1):
  from fontTools.ttLib import TTFont
  font = TTFont(font)
  glyph_set = font.getGlyphSet()
  glyph_name = font.getBestCmap()[ord(char)]
  glyph = glyph_set[glyph_name]

  from fontTools.pens.boundsPen import BoundsPen
  bounds = BoundsPen(glyph_set)
  glyph.draw(bounds)
  gx0, gy0, gx1, gy1 = bounds.bounds

  from fontTools.misc.transform import Transform
  scale = min((x1 - x0) / (gx1 - gx0), (y1 - y0) / (gy1 - gy0))
  transform = Transform(
    scale, 0, 0, scale,
    (x0 + x1 - scale * (gx0 + gx1)) / 2,
    (y0 + y1 - scale * (gy0 + gy1)) / 2)

  from fontTools.pens.recordingPen import DecomposingRecordingPen
  from fontTools.pens.transformPen import TransformPen
  recording = DecomposingRecordingPen(glyph_set)
  glyph.draw(recording)
  recording.replay(TransformPen(pen, transform))

upm         = 1000  # units per em
ascent      =  800
descent     = -100
glyph_width =  600
advance     =  720

fb = FontBuilder(upm, isTTF=True)

pen = TTGlyphPen(None)
x0, x1 = 0, glyph_width
y0, y1 = descent, ascent
stroke = 50
outer_radius = 200
inner_radius = outer_radius - stroke

draw_rounded_rect(pen, x0, y0, x1, y1, outer_radius, clockwise=True)
draw_rounded_rect(pen, x0 + stroke, y0 + stroke, x1 - stroke, y1 - stroke, inner_radius, clockwise=False)

padding = 120
draw_glyph(
  pen, '/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Bold.ttf', '?',
  x0 + stroke + padding, y0 + stroke + padding,
  x1 - stroke - padding, y1 - stroke - padding,
)

fb.setupGlyphOrder(['.notdef'])
fb.setupCharacterMap({})
fb.setupGlyf({'.notdef': pen.glyph()})
fb.setupHorizontalMetrics({'.notdef': (advance, (advance - glyph_width) // 2)})
fb.setupHorizontalHeader(ascent=ascent, descent=descent)
fb.setupNameTable({
  'familyName': 'Notdef Tofu',
  'uniqueFontIdentifier': 'NotdefTofu-Regular',
})
fb.setupOS2(
  sTypoAscender=ascent, sTypoDescender=descent,
  usWinAscent=ascent, usWinDescent=-descent,
)
import os
from fontTools.misc.timeTools import epoch_diff
build_timestamp = int(os.environ.get('SOURCE_DATE_EPOCH', 0)) - epoch_diff
fb.font['head'].created = build_timestamp
fb.font['head'].modified = build_timestamp
fb.setupPost()
fb.save('notdef-tofu.ttf')
