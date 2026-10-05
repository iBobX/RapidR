xof 0303txt 0032
// A DirectX .X model for tests/fixtures/d3d_xfile.bas: two squares in a
// frame moved 1.5 to the left, the first red (a material named at the top,
// referenced), the second white with tests/fixtures/d3d_tex.bmp on it.

template Material {
 <3D82AB4D-62DA-11cf-AB39-0020AF71E433>
 ColorRGBA faceColor;
 FLOAT power;
 ColorRGB specularColor;
 ColorRGB emissiveColor;
 [...]
}

Header {
 1;
 0;
 1;
}

Material RedMat {
 1.000000;0.000000;0.000000;1.000000;;
 0.000000;
 0.000000;0.000000;0.000000;;
 0.000000;0.000000;0.000000;;
}

Frame Model {
 FrameTransformMatrix {
  1.000000,0.000000,0.000000,0.000000,
  0.000000,1.000000,0.000000,0.000000,
  0.000000,0.000000,1.000000,0.000000,
  -1.500000,0.000000,0.000000,1.000000;;
 }

 Mesh Squares {
  8;
  -1.000000;1.000000;0.000000;,
  1.000000;1.000000;0.000000;,
  1.000000;-1.000000;0.000000;,
  -1.000000;-1.000000;0.000000;,
  2.000000;1.000000;0.000000;,
  4.000000;1.000000;0.000000;,
  4.000000;-1.000000;0.000000;,
  2.000000;-1.000000;0.000000;;
  2;
  4;0,1,2,3;,
  4;4,5,6,7;;

  MeshNormals {
   1;
   0.000000;0.000000;-1.000000;;
   2;
   4;0,0,0,0;,
   4;0,0,0,0;;
  }

  MeshMaterialList {
   2;
   2;
   0,
   1;;
   { RedMat }
   Material {
    1.000000;1.000000;1.000000;1.000000;;
    0.000000;
    0.000000;0.000000;0.000000;;
    0.000000;0.000000;0.000000;;
    TextureFilename {
     "d3d_tex.bmp";
    }
   }
  }

  MeshTextureCoords {
   8;
   0.000000;0.000000;,
   1.000000;0.000000;,
   1.000000;1.000000;,
   0.000000;1.000000;,
   0.000000;0.000000;,
   1.000000;0.000000;,
   1.000000;1.000000;,
   0.000000;1.000000;;
  }
 }
}
