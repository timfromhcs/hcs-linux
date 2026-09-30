# CPU Image Generation Studio Guide

txt2img + img2img on pure CPU (SD 1.5 LCM Q4, 512x512 in 4-8 steps,
15-25s on quad-core, <=2.2GB peak). CLI: `hcs image "..." --steps 6 -o
render.png`. GUI: Image Studio tab in hcs-chat. On-demand lifecycle only.
