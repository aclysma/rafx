# For now create the basis file outside of rafx/distill as it
# requires reading data from multiple files for a single
# asset, and this isn't supported yet.

# In order to get correct arrangements, orient the images like
#  2
# 1405
#  3
#
# 0: Right (+X)
# 1: Left (-X)
# 2: Top (+Y)
# 3: Bottom (-Y)
# 4: Front (+Z)
# 5: Back (-Z)  
#
# - Mipmaps may cause seams in the skybox (but no mips can give 
#   nasty sampling effects on some high-frequency images like
#   star fields)
# - Use clamp to edge sampling

basisu -mipmap -uastc -tex_type cubemap right.png left.png top.png bot.png front.png back.png -output_file cubemap.basis
cp cubemap.basis ../../assets/textures/skybox.basis