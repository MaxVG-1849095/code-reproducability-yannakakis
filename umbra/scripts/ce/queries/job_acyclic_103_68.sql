select count(*) from imdb117, imdb73, imdb5 where imdb117.d = imdb73.s and imdb73.s = imdb5.s;