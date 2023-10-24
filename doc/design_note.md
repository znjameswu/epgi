# Pixel perfect UI
Or: Pixel-snapping, physical pixel alignment

Related: Fractional scaling

1. Idea 1: We can easily achieve RenderObject-controlled pixel-snapping by using affine transform from the pixel canvas, while guaranteeing any caching layer to be pixel-snapping.
    2. Problem: Snapping failed when cached layer gets scaled.
    3. Problem: Unnecessary snapping when cached layer gets rotated.




# 2D transformation terminology

Orthognal transformation: rotation + reflection

Similarity transformation: rotation + scaling + reflection,  No shear