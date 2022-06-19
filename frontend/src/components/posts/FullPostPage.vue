<template>
    <v-container>
        <v-layout align-center>
            <v-card
                color="secondary"
                min-height="1000"
                width="80%"
                elevation="4"
                class="justify-center"
            >
                <v-app-bar elevation="0" color="secondary" class="py-0">
                    <v-btn
                        small
                        plain
                        text
                        color="primary"
                        @click="redirrectCommunity()"
                        ><b>{{ post.community }}</b></v-btn
                    >
                    <div class="overline accent--text">
                        {{ getCreatedDate() }}
                    </div>
                </v-app-bar>

                <v-card-title>
                    <p class="headling font-weight-bold white--text">
                        {{ post.title }}
                    </p>
                </v-card-title>
                <v-card class="mx-5" color="white">
                    <div v-for="content in post.content" :key="content">
                        <div v-if="content.text">
                            <v-card-text class="text-left">
                                <div class="white--text">
                                    {{ post.content[0].text.text }}
                                </div>
                            </v-card-text>
                        </div>
                        <div v-else-if="content.markdown">
                            <VueMarkdown class="mx-2">{{
                                content.markdown.text
                            }}</VueMarkdown>
                        </div>
                        <div v-else-if="content.image">
                            <!-- here we will have image content displayed -->
                            <!-- (hopefully) allows image/text to displayed sequentially -->
                        </div>
                    </div>
                </v-card>
                <v-card-actions>
                    <v-list-item>
                        <v-btn color="secondary" @click="redirrectUserPage()">
                            <v-list-item-avatar color="grey darken-3">
                                <img
                                    class="elevation-10"
                                    outlined
                                    src="../../assets/defaultUserIcon.jpg"
                                />
                            </v-list-item-avatar>

                            <v-list-item-content align="left">
                                <v-list-item-title class="white--text">{{
                                    post.author.username
                                }}</v-list-item-title>
                            </v-list-item-content>
                        </v-btn>
                        <v-list-item align="left">
                            <v-btn icon
                                ><v-icon color="accent"
                                    >mdi-thumb-up</v-icon
                                ></v-btn
                            >
                            <!-- </v-list-item>
                        <v-list-item align="left"> -->
                            <v-btn icon
                                ><v-icon color="accent"
                                    >mdi-thumb-down</v-icon
                                ></v-btn
                            >
                        </v-list-item>

                        <v-row justify="end">
                            <v-btn @click="overlay = !overlay" icon large>
                                <v-icon color="accent">
                                    mdi-reply
                                </v-icon>
                            </v-btn>
                        </v-row>
                    </v-list-item>
                </v-card-actions>
                <v-card-subtitle class="text-left white--text"
                    >Comments:</v-card-subtitle
                >
                <Comment
                    v-for="comment in post.children"
                    :key="comment.id"
                    :comment="comment"
                    v-on:postSuccesful="refreshReplies"
                    v-on:deleteComment="refreshReplies"
                ></Comment>
            </v-card>
            <v-overlay :z-index="zIndex" :value="overlay">
                <ReplyBox
                    v-on:closeBox="overlay = false"
                    v-on:postSuccesful="refreshReplies"
                    :item="post"
                />
            </v-overlay>
        </v-layout>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </v-container>
</template>
<script>
import Comment from './comment.vue'
import ReplyBox from './ReplyBox.vue'
import VueMarkdown from 'vue-markdown'

export default {
    mounted() {
        this.getPost()
    },
    data() {
        return {
            postID: this.$route.params.postid,
            url: '/internal/posts',
            overlay: false,
            zIndex: 10,
            post: {
                community: '',
                title: '',
                content: [],
            },
            loading: false,
            commentButtonText: 'See Comments',

            // error handling
            errorMessage: '',
            errorSnackbar: false,
        }
    },
    components: {
        Comment,
        ReplyBox,
        VueMarkdown,
    },
    methods: {
        getPost() {
            const url = `${this.url}/${this.postID}`

            this.$http
                .get(url, {}, { withCredentials: true })
                .then(response => {
                    const post = response.data
                    this.post = post
                })
                .catch(error => {
                    this.errorMessage = 'Problem contacting the server'
                    this.errorSnackbar = true
                })
        },

        refreshReplies() {
            this.overlay = false
            this.getPost()
        },

        getCreatedDate() {
            const d = new Date(this.post.created)
            return d.toUTCString()
        },

        redirrectCommunity() {
            const url = `/community/${this.post.community}`
            this.$router.push(url)
        },

        redirrectUserPage() {
            const url = `/user/${this.post.author.username}`
            this.$router.push(url)
        },
    },
}
</script>
