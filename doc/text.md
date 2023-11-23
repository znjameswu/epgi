# Linebreak algorithm
1. Linebreak at least depends on pairs
2. Linebreak is described as a procedural spec
3. Linebreak's impl is based on pair table
4. Linebreak logic is beyond our layout protocol

https://www.unicode.org/reports/tr14/tr14-17.html#PairBasedImplementation

# Text overflow Ellipsis
Flutter designated this to the TextPainter, which was ultimately encoded in ParagraphStyle and handled by the canvas impl