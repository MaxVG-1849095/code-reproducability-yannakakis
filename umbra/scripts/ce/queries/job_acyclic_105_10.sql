select count(*) from imdb100, imdb124, imdb13, imdb18 where imdb100.d = imdb124.d and imdb124.d = imdb13.s and imdb13.s = imdb18.s;