select count(*) from imdb2, imdb58, imdb76 where imdb2.d = imdb58.s and imdb58.s = imdb76.s;